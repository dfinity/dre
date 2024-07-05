use clap::Args;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Get {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Get {
    fn require_neuron() -> bool {
        false
    }

    fn require_registry() -> bool {
        false
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
