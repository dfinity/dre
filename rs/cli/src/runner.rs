use crate::cli::Opts;
use crate::clients;
use crate::ic_admin;
use crate::ops_subnet_node_replace;
use decentralization::SubnetChangeResponse;
use dialoguer::Confirm;
use futures::Future;
use ic_base_types::PrincipalId;
use log::warn;
use mercury_management_types::{TopologyProposal, TopologyProposalKind, TopologyProposalStatus};
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct Runner {
    ic_admin: ic_admin::Cli,
    dashboard_backend_client: clients::DashboardBackendClient,
}

impl Runner {
    pub fn deploy(&self, subnet: &PrincipalId, version: &str) -> anyhow::Result<()> {
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

    pub async fn membership_replace(
        &self,
        request: mercury_management_types::requests::MembershipReplaceRequest,
    ) -> anyhow::Result<()> {
        let change = self.dashboard_backend_client.membership_replace(request).await?;
        self.swap_nodes(change).await
    }

    async fn swap_nodes(&self, change: SubnetChangeResponse) -> anyhow::Result<()> {
        println!("{}", change);

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
                format!(
                    "Please finalize the last replacement first\n\n\t{} subnet --id {subnet_id} replace --finalize\n",
                    std::env::args().next().unwrap_or_else(|| "release-cli".to_string())
                )
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

        let add_proposal_id = if !self.ic_admin.dry_run {
            self.wait_for_replacement_nodes(subnet_id).await?.0.id
        } else {
            const DUMMY_ID: u64 = 1234567890;
            warn!("Set the first proposal ID to a dummy value: {}", DUMMY_ID);
            DUMMY_ID
        };
        self.run_finalize_swap(change, add_proposal_id).await
    }

    async fn run_finalize_swap(&self, change: SubnetChangeResponse, add_proposal_id: u64) -> anyhow::Result<()> {
        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::RemoveNodesFromSubnet {
                    nodes: change.removed.clone(),
                },
                ops_subnet_node_replace::replace_proposal_options(&change, add_proposal_id.into())?.with_motivation(format!("Finalize the replacements started with proposal [{add_proposal_id}](https://dashboard.internetcomputer.org/proposal/{add_proposal_id})")),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    async fn run_cancel_swap(
        &self,
        change: SubnetChangeResponse,
        add_proposal_id: u64,
        motivation: String,
    ) -> anyhow::Result<()> {
        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::RemoveNodesFromSubnet {
                    nodes: change.added.clone(),
                },
                ops_subnet_node_replace::cancel_replace_proposal_options(&change, add_proposal_id)?
                    .with_motivation(motivation),
            )
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    async fn wait_for_replacement_nodes(
        &self,
        subnet_id: PrincipalId,
    ) -> anyhow::Result<(TopologyProposal, SubnetChangeResponse)> {
        Ok(loop {
            if let Ok(Some(proposal)) = self.dashboard_backend_client.subnet_pending_action(subnet_id).await {
                if let TopologyProposal {
                    status: TopologyProposalStatus::Executed,
                    kind: TopologyProposalKind::ReplaceNode(replace),
                    ..
                } = &proposal
                {
                    break (
                        proposal.clone(),
                        SubnetChangeResponse {
                            added: replace.new_nodes.clone(),
                            removed: replace.old_nodes.clone(),
                            subnet_id: subnet_id.into(),
                            ..Default::default()
                        },
                    );
                }
            }
            sleep(Duration::from_secs(10)).await;
        })
    }

    pub async fn recover_finalize_swap(&self, subnet_id: PrincipalId) -> anyhow::Result<()> {
        let (topology_proposal, change) = self.wait_for_replacement_nodes(subnet_id).await?;

        self.with_confirmation(|r| {
            let change = change.clone();
            let add_proposal_id = topology_proposal.id;
            async move { r.run_finalize_swap(change, add_proposal_id).await }
        })
        .await
    }

    pub async fn cancel_swap(&self, subnet_id: PrincipalId, motivation: String) -> anyhow::Result<()> {
        let (topology_proposal, change) = self.wait_for_replacement_nodes(subnet_id).await?;

        self.with_confirmation(|r| {
            let change = change.clone();
            let add_proposal_id = topology_proposal.id;
            let motivation = motivation.clone();
            async move { r.run_cancel_swap(change, add_proposal_id, motivation).await }
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

    fn dry(&self) -> Self {
        Self {
            ic_admin: self.ic_admin.dry_run(),
            dashboard_backend_client: self.dashboard_backend_client.clone(),
        }
    }

    pub async fn from_opts(cli_opts: &Opts) -> anyhow::Result<Self> {
        Ok(Self {
            ic_admin: ic_admin::Cli::from_opts(cli_opts).await?,
            dashboard_backend_client: clients::DashboardBackendClient::new(cli_opts.network.clone(), cli_opts.dev),
        })
    }
}
