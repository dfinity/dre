use clap::Args;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Analyze {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Analyze {
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
