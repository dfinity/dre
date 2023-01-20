use crate::cli::Opts;
use crate::clients;
use crate::ic_admin;
use crate::ic_admin::ProposeOptions;
use crate::ops_subnet_node_replace;
use decentralization::SubnetChangeResponse;
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

    pub async fn subnet_extend(
        &self,
        request: ic_management_types::requests::SubnetExtendRequest,
        motivation: String,
        verbose: bool,
    ) -> anyhow::Result<()> {
        let subnet = request.subnet;
        let change = self.dashboard_backend_client.subnet_extend(request).await?;
        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        self.run_membership_change(
            change,
            ProposeOptions {
                title: format!("Extend subnet {subnet}").into(),
                summary: format!("Extend subnet {subnet}").into(),
                motivation: motivation.clone().into(),
            },
        )
        .await
    }

    pub async fn membership_replace(
        &self,
        request: ic_management_types::requests::MembershipReplaceRequest,
        verbose: bool,
    ) -> anyhow::Result<()> {
        let change = self.dashboard_backend_client.membership_replace(request).await?;
        if verbose {
            if let Some(run_log) = &change.run_log {
                println!("{}\n", run_log.join("\n"));
            }
        }
        println!("{}", change);

        self.run_membership_change(
            change.clone(),
            ops_subnet_node_replace::replace_proposal_options(&change)?,
        )
        .await
    }

    async fn run_membership_change(&self, change: SubnetChangeResponse, options: ProposeOptions) -> anyhow::Result<()> {
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
                options,
            )
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn from_opts(cli_opts: &Opts) -> anyhow::Result<Self> {
        Ok(Self {
            ic_admin: ic_admin::Cli::from_opts(cli_opts, true).await?,
            dashboard_backend_client: clients::DashboardBackendClient::new(cli_opts.network.clone(), cli_opts.dev),
        })
    }
}
