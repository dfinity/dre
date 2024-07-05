use clap::Args;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Rescue {
    /// Node features or Node IDs to exclude from the replacement
    #[clap(long, num_args(1..))]
    pub keep_nodes: Option<Vec<String>>,
}

impl ExecutableCommand for Rescue {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
