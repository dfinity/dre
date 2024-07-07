use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Deploy {
    /// Version to propose for the subnet
    #[clap(long, short)]
    pub version: String,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,
}

impl ExecutableCommand for Deploy {
    fn require_neuron(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner();
        runner.deploy(&self.id, &self.version).await
    }
}
