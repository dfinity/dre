use std::path::PathBuf;

use clap::Args;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct DerToPrincipal {
    /// Path to the DER file
    pub path: PathBuf,
}

impl ExecutableCommand for DerToPrincipal {
    fn require_neuron(&self) -> bool {
        false
    }

    fn require_registry(&self) -> bool {
        false
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
