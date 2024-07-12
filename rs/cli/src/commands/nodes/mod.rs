use clap::{Args, Subcommand};
use remove::Remove;

use super::{ExecutableCommand, IcAdminRequirement};

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
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            NodesSubcommands::Remove(r) => r.require_ic_admin(),
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
