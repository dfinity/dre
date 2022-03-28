use clap::{CommandFactory, ErrorKind, Parser};
use colored::Colorize;
use decentralization::SubnetChangeResponse;
use dialoguer::Confirm;
use dotenv::dotenv;
use futures::Future;
use ic_base_types::PrincipalId;
use log::{info, warn};
use mercury_management_types::{TopologyProposal, TopologyProposalKind, TopologyProposalStatus};
use tokio::time::{sleep, Duration};
mod cli;
mod clients;
pub(crate) mod defaults;
mod ic_admin;
mod ops_subnet_node_replace;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_env();

    let cli_opts = cli::Opts::parse();
    init_logger();

    ic_admin::with_ic_admin(Default::default(), async {
        let runner = Runner {
            ic_admin: ic_admin::Cli::from(&cli_opts),
            dashboard_backend_client: clients::DashboardBackendClient {
                url: cli_opts.backend_url.clone(),
            },
        };

        // Start of actually doing stuff with commands.
        match &cli_opts.subcommand {
            cli::Commands::DerToPrincipal { path } => {
                let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
                println!("{}", principal);
                Ok(())
            }

            cli::Commands::Subnet(subnet) => {
                let mut cmd = cli::Opts::command();
                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { .. } => {
                        if subnet.id.is_none() {
                            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `id` not found")
                                .exit();
                        }
                    }
                    cli::subnet::Commands::Replace { nodes, finalize, .. } => {
                        if *finalize {
                            if subnet.id.is_none() {
                                cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `id` not found")
                                    .exit();
                            } else if !nodes.is_empty() {
                                cmd.error(
                                    ErrorKind::ArgumentConflict,
                                    "Cannot pass `nodes` when finalizing a replacement",
                                )
                                .exit();
                            }
                        } else if !nodes.is_empty() && subnet.id.is_some() {
                            cmd.error(
                                ErrorKind::ArgumentConflict,
                                "Specify either a subnet id or a list of nodes to replace",
                            )
                            .exit();
                        } else if nodes.is_empty() && subnet.id.is_none() {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Specify either a subnet id or a list of nodes to replace",
                            )
                            .exit();
                        }
                    }
                }

                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { version } => runner.deploy(&subnet.id.unwrap(), version),
                    cli::subnet::Commands::Replace {
                        nodes,
                        no_heal,
                        optimize,
                        motivation,
                        finalize,
                        exclude,
                    } => {
                        if *finalize {
                            runner.recover_finalize_swap(subnet.id.unwrap()).await
                        } else {
                            runner
                                .membership_replace(mercury_management_types::requests::MembershipReplaceRequest {
                                    target: match &subnet.id {
                                        Some(subnet) => {
                                            mercury_management_types::requests::ReplaceTarget::Subnet(*subnet)
                                        }
                                        None => {
                                            if let Some(motivation) = motivation.clone() {
                                                mercury_management_types::requests::ReplaceTarget::Nodes {
                                                    nodes: nodes.clone(),
                                                    motivation,
                                                }
                                            } else {
                                                cmd.error(
                                                    ErrorKind::MissingRequiredArgument,
                                                    "Required argument `motivation` not found",
                                                )
                                                .exit();
                                            }
                                        }
                                    },
                                    heal: !no_heal,
                                    optimize: *optimize,
                                    exclude: exclude.clone().into(),
                                })
                                .await
                        }
                    }
                }
            }

            cli::Commands::Get { args } => runner.ic_admin_generic_get(args),

            cli::Commands::Propose { args } => runner.ic_admin_generic_propose(args),
        }
    })
    .await
}

#[derive(Clone)]
pub struct Runner {
    ic_admin: ic_admin::Cli,
    dashboard_backend_client: clients::DashboardBackendClient,
}

impl Runner {
    fn deploy(&self, subnet: &PrincipalId, version: &str) -> anyhow::Result<()> {
        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::UpdateSubnetReplicaVersion {
                    subnet: *subnet,
                    version: version.to_string(),
                },
                ic_admin::ProposeOptions {
                    title: format!("Update subnet {subnet} to replica version {version}").into(),
                    summary: format!("Update subnet {subnet} to replica version {version}").into(),
                    motivation: None,
                },
            )
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    async fn membership_replace(
        &self,
        request: mercury_management_types::requests::MembershipReplaceRequest,
    ) -> anyhow::Result<()> {
        let change = self.dashboard_backend_client.membership_replace(request).await?;
        self.swap_nodes(change).await
    }

    fn print_before_after_scores(change: &SubnetChangeResponse) {
        println!("Decentralization score changes:");
        let before_individual = change.score_before.individual();
        let after_individual = change.score_after.individual();
        change
            .score_before
            .individual()
            .keys()
            .map(|k| {
                let before = before_individual.get(k).unwrap();
                let after = after_individual.get(k).unwrap();
                let output = format!(
                    "{}: {:.2} -> {:.2}  {:>7}",
                    k,
                    before,
                    after,
                    format!("({:+.0}%)", ((after - before) / before) * 100.)
                );
                if before > after {
                    output.bright_red()
                } else if after > before {
                    output.bright_green()
                } else {
                    output.dimmed()
                }
            })
            .for_each(|s| println!("{: >40}", s));

        let total_before = change.score_before.total();
        let total_after = change.score_after.total();
        let output = format!(
            "\tTotal: {:.2} -> {:.2}  ({:+.0}%)",
            total_before,
            total_after,
            ((total_after - total_before) / total_before) * 100.
        )
        .bold();
        println!(
            "\n{}\n",
            if total_before > total_after {
                output.red()
            } else if total_after > total_before {
                output.green()
            } else {
                output.dimmed()
            }
        );
    }

    async fn swap_nodes(&self, change: SubnetChangeResponse) -> anyhow::Result<()> {
        Self::print_before_after_scores(&change);

        self.with_confirmation(|r| {
            let change = change.clone();
            async move { r.run_swap_nodes(change).await }
        })
        .await
    }

    async fn run_swap_nodes(&self, change: SubnetChangeResponse) -> anyhow::Result<()> {
        let subnet_id = change
            .subnet_id
            .ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;
        let pending_action = self.dashboard_backend_client.subnet_pending_action(subnet_id).await?;
        if let Some(proposal) = pending_action {
            return Err(anyhow::anyhow!(vec![
                format!(
                    "There is a pending proposal for this subnet: https://dashboard.internetcomputer.org/proposal/{}",
                    proposal.id
                ),
                "Please complete it first by running `release-cli subnet --subnet-id {subnet_id} replace --finalize`"
                    .to_string(),
            ]
            .join("\n")));
        }

        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::AddNodesToSubnet {
                    subnet_id,
                    nodes: change.added.clone(),
                },
                ops_subnet_node_replace::replace_proposal_options(&change, None)?,
            )
            .map_err(|e| anyhow::anyhow!(e))?;

        self.run_finalize_swap(change).await
    }

    async fn run_finalize_swap(&self, change: SubnetChangeResponse) -> anyhow::Result<()> {
        let subnet_id = change
            .subnet_id
            .ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?;

        let add_proposal_id = if !self.ic_admin.dry_run {
            loop {
                if let Ok(Some(proposal)) = self.dashboard_backend_client.subnet_pending_action(subnet_id).await {
                    if matches!(proposal.status, TopologyProposalStatus::Executed) {
                        break proposal.id;
                    }
                }
                sleep(Duration::from_secs(10)).await;
            }
        } else {
            const DUMMY_ID: u64 = 1234567890;
            warn!("Set the first proposal ID to a dummy value: {}", DUMMY_ID);
            DUMMY_ID
        }
        .into();

        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::RemoveNodesFromSubnet {
                    nodes: change.removed.clone(),
                },
                ops_subnet_node_replace::replace_proposal_options(&change, add_proposal_id)?,
            )
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    async fn recover_finalize_swap(&self, subnet_id: PrincipalId) -> anyhow::Result<()> {
        let change = loop {
            if let Ok(proposal) = self.dashboard_backend_client.subnet_pending_action(subnet_id).await {
                if let TopologyProposal {
                    status: TopologyProposalStatus::Executed,
                    kind: TopologyProposalKind::ReplaceNode(replace),
                    id,
                } = proposal.ok_or_else(|| anyhow::anyhow!("No pending proposal found for this subnet"))?
                {
                    break SubnetChangeResponse {
                        added: replace.new_nodes,
                        removed: replace.old_nodes,
                        subnet_id: subnet_id.into(),
                        motivation: format!("Finalize the replacements started with proposal {}", id).into(),
                        ..Default::default()
                    };
                };
            }
            sleep(Duration::from_secs(10)).await
        };

        self.with_confirmation(|r| {
            let change = change.clone();
            async move { r.run_finalize_swap(change).await }
        })
        .await
    }

    async fn with_confirmation<E, F>(&self, exec: E) -> anyhow::Result<()>
    where
        E: Fn(Self) -> F,
        F: Future<Output = anyhow::Result<()>>,
    {
        if !self.ic_admin.dry_run {
            exec(self.dry()).await?;
            if !Confirm::new()
                .with_prompt("Do you want to continue?")
                .default(false)
                .interact()?
            {
                return Err(anyhow::anyhow!("Action aborted"));
            }
        }

        exec(self.clone()).await
    }

    fn ic_admin_generic_get(&self, args: &[String]) -> anyhow::Result<()> {
        self.ic_admin.run_passthrough_get(args)
    }

    fn ic_admin_generic_propose(&self, args: &[String]) -> anyhow::Result<()> {
        self.ic_admin.run_passthrough_propose(args)
    }

    fn dry(&self) -> Self {
        Self {
            ic_admin: self.ic_admin.dry_run(),
            dashboard_backend_client: self.dashboard_backend_client.clone(),
        }
    }
}

fn init_env() {
    if dotenv().is_err() {
        info!(".env file not found. You can copy env.template to .env to adjust configuration.");
    };
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Set a default logging level: info, if nothing else specified in environment
                // variables RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
