use clap::{Args, Subcommand};
use remove::Remove;

use super::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

mod remove;

#[derive(Args, Debug)]
pub struct Nodes {
    #[clap(subcommand)]
    pub subcommand: NodesSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum NodesSubcommands {
    /// Remove nodes from the network
    Remove(Remove),
}

impl ExecutableCommand for Nodes {
    fn require_neuron(&self) -> IcAdminRequirement {
        match &self.subcommand {
            NodesSubcommands::Remove(r) => r.require_neuron(),
        }
    }

    fn require_registry(&self) -> RegistryRequirement {
        match &self.subcommand {
            NodesSubcommands::Remove(r) => r.require_registry(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            NodesSubcommands::Remove(r) => r.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            NodesSubcommands::Remove(r) => r.validate(cmd),
        }
    }
}
