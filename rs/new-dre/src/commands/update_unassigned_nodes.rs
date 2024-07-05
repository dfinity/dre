use clap::Args;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,
}

impl ExecutableCommand for UpdateUnassignedNodes {
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
