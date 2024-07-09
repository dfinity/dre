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
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
