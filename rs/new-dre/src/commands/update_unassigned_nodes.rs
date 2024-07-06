use clap::Args;

use super::{ExecutableCommand, NeuronRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,
}

impl ExecutableCommand for UpdateUnassignedNodes {
    fn require_neuron(&self) -> NeuronRequirement {
        NeuronRequirement::Hardcoded
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
