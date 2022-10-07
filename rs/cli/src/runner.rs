use crate::cli::Opts;
use crate::clients;
use crate::ic_admin;
use crate::ops_subnet_node_replace;
use decentralization::SubnetChangeResponse;
use dialoguer::Confirm;
use futures::Future;
use ic_base_types::PrincipalId;

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
        request: ic_management_types::requests::MembershipReplaceRequest,
    ) -> anyhow::Result<()> {
        let change = self.dashboard_backend_client.membership_replace(request).await?;
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
            return Err(anyhow::anyhow!(format!(
                "There is a pending proposal for this subnet: https://dashboard.internetcomputer.org/proposal/{}",
                proposal.id
            )));
        }

        self.ic_admin
            .propose_run(
                ic_admin::ProposeCommand::ChangeSubnetMembership {
                    subnet_id,
                    node_ids_add: change.added.clone(),
                    node_ids_remove: change.removed.clone(),
                },
                ops_subnet_node_replace::replace_proposal_options(&change)?,
            )
            .map_err(|e| anyhow::anyhow!(e))
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
            ic_admin: ic_admin::Cli::from_opts(cli_opts, true).await?,
            dashboard_backend_client: clients::DashboardBackendClient::new(cli_opts.network.clone(), cli_opts.dev),
        })
    }
}
