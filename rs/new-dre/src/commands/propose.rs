use clap::Args;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Propose {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Propose {
    fn require_neuron() -> bool {
        true
    }

    fn require_registry() -> bool {
        false
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
