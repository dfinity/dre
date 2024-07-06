use clap::Args;

use crate::commands::{ExecutableCommand, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Deploy {
    /// Version to propose for the subnet
    #[clap(long, short)]
    pub version: String,
}

impl ExecutableCommand for Deploy {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
