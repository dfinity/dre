use crate::auth::Neuron;
use crate::proposal_executors::ProducesProposalResult;
use crate::proposal_executors::ProposalResponseWithId;
use crate::proposal_executors::RunnableViaIcAdmin;
use crate::util::run_capturing_stdout;
use anyhow::anyhow;
use colored::Colorize;
use futures::future::BoxFuture;
use ic_base_types::PrincipalId;
use ic_management_types::{Artifact, Network};
use log::debug;
use log::info;
use mockall::automock;
use regex::Regex;
use shlex::try_quote;
use std::fmt::Debug;
use strum::Display as StrumDisplay;
use tokio::process::Command;
use url::Url;

const MAX_SUMMARY_CHAR_COUNT: usize = 29000;
const PROPOSE_CMD_PREFIX: &str = "propose-to-";
const GET_CMD_PREFIX: &str = "get-";

#[automock]
// automock complains without the explicit allow below
#[allow(elided_named_lifetimes)]
pub trait IcAdmin: Send + Sync + Debug {
    fn ic_admin_path(&self) -> Option<String>;

    /// Runs the proposal in simulation mode (--dry-run).  Prints out the result.
    fn simulate_proposal(&self, cmd: Vec<String>) -> BoxFuture<'_, anyhow::Result<()>>;

    /// Runs the proposal in forrealz mode.  Result is returned and logged at debug level.
    fn submit_proposal<'a, 'b>(&'a self, cmd: Vec<String>, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<String>>
    where
        'a: 'b;

    fn grep_subcommand_arguments(&self, subcommand: &str) -> BoxFuture<'_, anyhow::Result<String>>;

    fn grep_subcommands<'a, 'b>(&'a self, needle_regex: &'a str) -> BoxFuture<'b, anyhow::Result<Vec<String>>>
    where
        'a: 'b;

    // /Runs a passthrough for an ic-admin get-* command.  Returns the result and logs it at debug level
    fn get<'a>(&'a self, args: &'a [String]) -> BoxFuture<'_, anyhow::Result<String>>;
}

#[derive(Clone, Debug)]
pub struct IcAdminImpl {
    network: Network,
    ic_admin_bin_path: Option<String>,
    neuron: Neuron,
}

impl IcAdmin for IcAdminImpl {
    fn ic_admin_path(&self) -> Option<String> {
        self.ic_admin_bin_path.clone()
    }

    /// Run ic-admin and parse sub-commands that it lists with "--help",
    /// extract the ones matching `needle_regex` and return them as a
    /// `Vec<String>`
    fn grep_subcommands<'a, 'b>(&'a self, needle_regex: &'a str) -> BoxFuture<'b, anyhow::Result<Vec<String>>>
    where
        'a: 'b,
    {
        Box::pin(async move {
            let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
            let cmd_result = Command::new(ic_admin_path).args(["--help"]).output().await;
            match cmd_result.map_err(|e| e.to_string()) {
                Ok(output) => {
                    if output.status.success() {
                        let cmd_stdout = String::from_utf8_lossy(output.stdout.as_ref());
                        let re = Regex::new(needle_regex).unwrap();
                        Ok(re
                            .captures_iter(cmd_stdout.as_ref())
                            .map(|capt| String::from(capt.get(1).expect("group 1 not found").as_str().trim()))
                            .collect())
                    } else {
                        Err(anyhow::anyhow!(
                            "Execution of ic-admin failed: {}",
                            String::from_utf8_lossy(output.stderr.as_ref())
                        ))
                    }
                }
                Err(err) => Err(anyhow::anyhow!("Error starting ic-admin process: {}", err)),
            }
        })
    }

    fn grep_subcommand_arguments<'a>(&'a self, subcommand: &str) -> BoxFuture<'a, anyhow::Result<String>> {
        let subcommand = subcommand.to_string();
        Box::pin(async move {
            let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
            let output = Command::new(ic_admin_path).args([subcommand.as_str(), "--help"]).output().await?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(output.stdout.as_ref()).to_string())
            } else {
                Err(anyhow::anyhow!(
                    "Execution of ic-admin failed: {}",
                    String::from_utf8_lossy(output.stderr.as_ref())
                ))
            }
        })
    }

    /// Run an `ic-admin get-*` command directly, and without an HSM
    fn get<'a>(&'a self, args: &'a [String]) -> BoxFuture<'a, anyhow::Result<String>> {
        Box::pin(async move {
            debug!("Running get {:?}.", args);
            if args.is_empty() {
                println!("List of available ic-admin 'get' sub-commands:\n");
                for subcmd in self.grep_subcommands(format!(r"\s+{}(.+?)\s", GET_CMD_PREFIX).as_str()).await? {
                    println!("\t{}", subcmd)
                }
                std::process::exit(0);
            }

            let nonoption_args = args.iter().enumerate().filter(|(_, arg)| !arg.starts_with("--")).collect::<Vec<_>>();
            // The `get` subcommand of the cli expects that "get-" prefix is not provided as
            // the ic-admin command
            let args = match nonoption_args.first() {
                None => args.to_vec(),
                Some(f) => {
                    if f.1.starts_with(GET_CMD_PREFIX) {
                        // The user did provide the "get-" prefix, so let's just keep it and use it.
                        // This provides a convenient backward compatibility with ic-admin commands
                        // i.e., `dre get get-subnet 0` still works, although `dre get
                        // subnet 0` is preferred
                        args.to_vec()
                    } else {
                        // But since ic-admin expects these commands to include the "get-" prefix, we
                        // need to add it back Example:
                        // `dre get subnet 0` becomes
                        // `ic-admin --nns-url "http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080" get-subnet 0`
                        let mut modified_args = args.to_vec();
                        modified_args[f.0] = String::from(GET_CMD_PREFIX) + f.1.as_str();
                        modified_args
                    }
                }
            };

            self.run(&args, false).await
        })
    }

    fn simulate_proposal(&self, cmd: Vec<String>) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            debug!("Simulating proposal {:?}.", cmd);
            let mut args = self.add_proposer(cmd);
            // Make sure there is no more than one `--dry-run` argument, or else ic-admin will complain.
            if !args.contains(&String::from("--dry-run")) {
                args.push("--dry-run".into())
            };
            self.run(args.as_slice(), true).await.map(|r| r.trim().to_string())?;
            Ok(())
        })
    }

    fn submit_proposal<'a, 'b>(&'a self, cmd: Vec<String>, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<String>>
    where
        'a: 'b,
    {
        Box::pin(async move {
            debug!("Submitting proposal {:?}.", cmd);
            let args = self.add_proposal_url(self.add_proposer(cmd), forum_post_link);
            self.run(args.as_slice(), false).await.map(|r| r.trim().to_string())
        })
    }
}

impl IcAdminImpl {
    pub fn new(network: Network, ic_admin_bin_path: Option<String>, neuron: Neuron) -> Self {
        Self {
            network,
            ic_admin_bin_path,
            neuron,
        }
    }

    // Run ic-admin command with the arguments specified, passing through standard output and error
    // to the calling process' standard output and error.  Standard input is unaffected.
    // If the command succeeds in running and returns 0 as exit code, it returns the standard output
    // as a (lossily-decoded) UTF-8 string as part of the Result returned by this function.
    fn run<'a>(&'a self, args: &'a [String], print_stdout: bool) -> BoxFuture<'a, anyhow::Result<String>> {
        let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
        let auth_options = self.neuron.as_arg_vec();
        let root_options = [auth_options, vec!["--nns-urls".to_string(), self.network.get_nns_urls_string()]].concat();

        Box::pin(async move {
            let mut cmd = Command::new(ic_admin_path);
            cmd.args([&root_options, args].concat());
            cmd.kill_on_drop(true);
            self.print_ic_admin_command_line(&cmd).await;
            let (output, result) = run_capturing_stdout(&mut cmd, print_stdout).await;
            match result {
                Ok(s) => {
                    if s.success() {
                        Ok(output)
                    } else {
                        Err(anyhow::anyhow!(
                            "ic-admin exited with non-zero exit code {}",
                            s.code().map(|c| c.to_string()).unwrap_or_else(|| "<none>".to_string()),
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("ic-admin execution failed before exiting: {}", e,)),
            }
        })
    }

    async fn print_ic_admin_command_line(&self, cmd: &Command) {
        info!(
            "Running ic-admin: \n$ {}{}",
            cmd.as_std().get_program().to_str().unwrap().yellow(),
            cmd.as_std()
                .get_args()
                .map(|s| s.to_str().unwrap().to_string())
                .fold("".to_string(), |acc, s| {
                    // If previous argument was pin it means that this one is the pin itself
                    if acc.ends_with("--pin") {
                        format!("{acc} <redacted>")
                    } else {
                        let s = if s.contains('\n') {
                            format!("'{}'", s.replace('\'', "'\\''"))
                        } else {
                            match try_quote(s.as_str()) {
                                Ok(sss) => sss.to_string(),
                                Err(_e) => s,
                            }
                        };
                        if s.starts_with("--") {
                            format!("{acc} \\\n    {s}")
                        } else if !acc.split(' ').last().unwrap_or_default().starts_with("--") {
                            format!("{acc} \\\n  {s}")
                        } else {
                            format!("{acc} {s}")
                        }
                    }
                })
                .yellow(),
        );
    }

    fn add_proposer(&self, args: Vec<String>) -> Vec<String> {
        [
            args,
            match self.neuron.maybe_proposer() {
                Some(proposer) => vec!["--proposer".to_string(), proposer],
                None => vec![],
            },
        ]
        .concat()
    }

    fn add_proposal_url(&self, args: Vec<String>, proposal_url: Option<Url>) -> Vec<String> {
        [
            args,
            match &proposal_url {
                Some(link) => vec!["--proposal-url".to_string(), link.to_string()],
                _ => vec![],
            },
        ]
        .concat()
    }
}

#[derive(StrumDisplay, Clone, Debug)]
#[strum(serialize_all = "kebab-case")]
pub enum IcAdminProposalCommand {
    ChangeSubnetMembership {
        subnet_id: PrincipalId,
        node_ids_add: Vec<PrincipalId>,
        node_ids_remove: Vec<PrincipalId>,
    },
    DeployGuestosToAllSubnetNodes {
        subnet: PrincipalId,
        version: String,
    },
    DeployGuestosToAllUnassignedNodes {
        replica_version: String,
    },
    DeployHostosToSomeNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    Raw(Vec<String>),
    RemoveNodes {
        nodes: Vec<PrincipalId>,
    },
    ReviseElectedVersions {
        release_artifact: Artifact,
        args: Vec<String>,
    },
    CreateSubnet {
        node_ids: Vec<PrincipalId>,
        replica_version: String,
        other_args: Vec<String>,
    },
    AddApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    RemoveApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
    },
    DeployGuestosToSomeApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    SetAuthorizedSubnetworks {
        subnets: Vec<PrincipalId>,
    },
}

impl IcAdminProposalCommand {
    fn args(&self) -> (String, Vec<String>) {
        let head: String = match &self {
            Self::Raw(args) => args
                .first()
                .map(|s| s.strip_prefix(PROPOSE_CMD_PREFIX).unwrap_or(s))
                .unwrap_or("")
                .to_string(),
            Self::ReviseElectedVersions { release_artifact, .. } => format!("revise-elected-{}-versions", release_artifact),
            Self::DeployGuestosToAllUnassignedNodes { .. } => "deploy-guestos-to-all-unassigned-nodes".to_string(),
            _ => self.to_string(),
        };
        let tail: Vec<String> = match &self {
            Self::ChangeSubnetMembership {
                subnet_id,
                node_ids_add: nodes_ids_add,
                node_ids_remove: nodes_ids_remove,
            } => [
                vec!["--subnet-id".to_string(), subnet_id.to_string()],
                if !nodes_ids_add.is_empty() {
                    [
                        vec!["--node-ids-add".to_string()],
                        nodes_ids_add.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                    ]
                    .concat()
                } else {
                    vec![]
                },
                if !nodes_ids_remove.is_empty() {
                    [
                        vec!["--node-ids-remove".to_string()],
                        nodes_ids_remove.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                    ]
                    .concat()
                } else {
                    vec![]
                },
            ]
            .concat(),
            Self::DeployGuestosToAllSubnetNodes { subnet, version } => {
                vec![subnet.to_string(), version.clone()]
            }
            Self::Raw(command_and_args) => match command_and_args.len() {
                0 => vec![],
                _ => command_and_args[1..].to_vec().clone(),
            },
            Self::DeployHostosToSomeNodes { nodes, version } => [
                nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec!["--hostos-version-id".to_string(), version.to_string()],
            ]
            .concat(),
            Self::RemoveNodes { nodes } => nodes.iter().map(|n| n.to_string()).collect(),
            Self::ReviseElectedVersions { args, .. } => args.clone(),
            Self::CreateSubnet {
                node_ids,
                replica_version,
                other_args,
            } => {
                let mut args = vec!["--subnet-type".to_string(), "application".to_string()];

                args.push("--replica-version-id".to_string());
                args.push(replica_version.to_string());

                for id in node_ids {
                    args.push(id.to_string())
                }
                args.extend(other_args.to_vec());
                args
            }
            Self::DeployGuestosToAllUnassignedNodes { replica_version } => vec!["--replica-version-id".to_string(), replica_version.clone()],
            Self::AddApiBoundaryNodes { nodes, version } => [
                nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
                vec!["--version".to_string(), version.to_string()],
            ]
            .concat(),
            Self::RemoveApiBoundaryNodes { nodes } => nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
            Self::DeployGuestosToSomeApiBoundaryNodes { nodes, version } => [
                nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
                vec!["--version".to_string(), version.to_string()],
            ]
            .concat(),
            Self::SetAuthorizedSubnetworks { subnets } => subnets.iter().flat_map(|s| ["--subnets".to_string(), s.to_string()]).collect::<Vec<_>>(),
        };
        (head, tail)
    }
}

#[derive(Default, Clone, Debug)]
pub struct IcAdminProposalOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IcAdminProposal {
    pub command: IcAdminProposalCommand,
    pub options: IcAdminProposalOptions,
}

impl IcAdminProposal {
    pub fn new(command: IcAdminProposalCommand, opts: IcAdminProposalOptions) -> Self {
        Self { command, options: opts }
    }
}

impl RunnableViaIcAdmin for IcAdminProposal {
    type Output = ProposalResponseWithId;

    fn to_ic_admin_arguments(&self) -> anyhow::Result<Vec<String>> {
        self.to_args()
    }
}

impl ProducesProposalResult for IcAdminProposal {
    type ProposalResult = ProposalResponseWithId;
}

impl IcAdminProposal {
    fn to_args(&self) -> anyhow::Result<Vec<String>> {
        if let Some(summary) = &self.options.summary {
            let summary_count = summary.chars().count();
            if summary_count > MAX_SUMMARY_CHAR_COUNT {
                return Err(anyhow!(
                    "Summary length {} exceeded MAX_SUMMARY_CHAR_COUNT {}",
                    summary_count,
                    MAX_SUMMARY_CHAR_COUNT,
                ));
            }
        }

        let (head, tail) = self.command.args();
        let head = match head.as_str() {
            "" => vec![],
            _ => vec![format!("{PROPOSE_CMD_PREFIX}{}", head)],
        };

        Ok([
            head,
            vec!["--silence-notices".to_string()], // Do not print notices since we are running from the automation
            self.options
                .clone()
                .title
                .map(|t| vec!["--proposal-title".to_string(), t])
                .unwrap_or_default(),
            self.options
                .clone()
                .summary
                .map(|s| {
                    vec![
                        "--summary".to_string(),
                        format!(
                            "{}{}",
                            s,
                            self.options
                                .clone()
                                .motivation
                                .map(|m| format!("\n\nMotivation: {m}"))
                                .unwrap_or_default(),
                        ),
                    ]
                })
                .unwrap_or_default(),
            tail,
        ]
        .concat())
    }
}
