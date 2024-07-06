use clap::Args;

use crate::commands::{ExecutableCommand, NeuronRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Remove {
    /// Skip removal of duplicate or dead nodes
    #[clap(long)]
    pub no_auto: bool,

    /// Remove also degraded nodes; by default only dead (offline) nodes are automatically removed
    #[clap(long)]
    pub remove_degraded: bool,

    /// Specifies the filter used to remove extra nodes
    pub extra_nodes_filter: Vec<String>,

    /// Features or Node IDs to not remove (exclude from the removal)
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Motivation for removing additional nodes
    #[clap(long, aliases = ["summary"])]
    pub motivation: Option<String>,
}

impl ExecutableCommand for Remove {
    fn require_neuron(&self) -> NeuronRequirement {
        NeuronRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::WithNodeDetails
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
