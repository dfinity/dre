use add::Add;
use clap::{Args, Subcommand};
use remove::Remove;
use update::Update;

use super::{ExecutableCommand, IcAdminRequirement};

mod add;
mod remove;
mod update;

#[derive(Args, Debug)]
pub struct ApiBoundaryNodes {
    #[clap(subcommand)]
    pub subcommand: ApiBoundaryNodesSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ApiBoundaryNodesSubcommands {
    /// Turn a set of unassigned nodes into API BNs
    Add(Add),

    #[clap(about = r#"Update specified set of nodes to the provided version.
The provided "version" must be already elected.
The "nodes" list must contain the node IDs where the version 
should be rolled out."#)]
    Update(Update),

    /// Decommission a set of API BNs and turn them again in unassigned nodes
    Remove(Remove),
}

impl ExecutableCommand for ApiBoundaryNodes {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            ApiBoundaryNodesSubcommands::Add(a) => a.require_ic_admin(),
            ApiBoundaryNodesSubcommands::Update(u) => u.require_ic_admin(),
            ApiBoundaryNodesSubcommands::Remove(r) => r.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            ApiBoundaryNodesSubcommands::Add(a) => a.execute(ctx).await,
            ApiBoundaryNodesSubcommands::Update(u) => u.execute(ctx).await,
            ApiBoundaryNodesSubcommands::Remove(r) => r.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            ApiBoundaryNodesSubcommands::Add(a) => a.validate(cmd),
            ApiBoundaryNodesSubcommands::Update(u) => u.validate(cmd),
            ApiBoundaryNodesSubcommands::Remove(r) => r.validate(cmd),
        }
    }
}