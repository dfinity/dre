use clap::Args;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Get {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Get {
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
