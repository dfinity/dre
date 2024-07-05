use clap::Args;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Pending {}

impl ExecutableCommand for Pending {
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
