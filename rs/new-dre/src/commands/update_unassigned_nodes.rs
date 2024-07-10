use clap::Args;
use ic_management_types::Network;

use crate::auth::Neuron;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,
}

impl ExecutableCommand for UpdateUnassignedNodes {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::OverridableBy {
            network: Network::mainnet_unchecked().unwrap(),
            neuron: Neuron::automation_neuron_unchecked(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin();

        let nns_subnet_id = match self.nns_subnet_id {
            Some(n) => n.to_owned(),
            None => {
                let res = ic_admin.run_passthrough_get(&["get-subnet-list".to_string()], true).await?;
                let subnet_list = serde_json::from_str::<Vec<String>>(&res)?;
                subnet_list.first().ok_or_else(|| anyhow::anyhow!("No subnet found"))?.clone()
            }
        };

        ic_admin.update_unassigned_nodes(&nns_subnet_id, &ctx.network()).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
