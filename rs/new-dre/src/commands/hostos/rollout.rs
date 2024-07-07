use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Rollout {
    /// HostOS version to be rolled out
    #[clap(long)]
    pub version: String,

    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..))]
    pub nodes: Vec<PrincipalId>,
}

impl ExecutableCommand for Rollout {
    fn require_neuron(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::WithNodeDetails
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
