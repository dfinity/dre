use clap::{Parser, Subcommand};
use create::Create;
use deploy::Deploy;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;

use super::{ExecutableCommand, IcAdminRequirement};

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;

#[derive(Parser, Debug)]
pub struct SubnetCommand {
    #[clap(subcommand)]
    pub subcommand: SubnetSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum SubnetSubcommand {
    /// Create a new proposal to rollout a new version to the subnet
    Deploy(Deploy),

    /// Replace the nodes in a subnet
    Replace(Replace),

    /// Resize the subnet
    Resize(Resize),

    /// Create a subnet
    Create(Create),

    /// Replace all nodes in a subnet except some nodes
    Rescue(Rescue),
}

impl ExecutableCommand for SubnetCommand {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            SubnetSubcommand::Deploy(d) => d.require_ic_admin(),
            SubnetSubcommand::Replace(r) => r.require_ic_admin(),
            SubnetSubcommand::Resize(r) => r.require_ic_admin(),
            SubnetSubcommand::Create(c) => c.require_ic_admin(),
            SubnetSubcommand::Rescue(r) => r.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            SubnetSubcommand::Deploy(d) => d.execute(ctx).await,
            SubnetSubcommand::Replace(r) => r.execute(ctx).await,
            SubnetSubcommand::Resize(r) => r.execute(ctx).await,
            SubnetSubcommand::Create(c) => c.execute(ctx).await,
            SubnetSubcommand::Rescue(r) => r.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            SubnetSubcommand::Deploy(d) => d.validate(cmd),
            SubnetSubcommand::Replace(r) => r.validate(cmd),
            SubnetSubcommand::Resize(r) => r.validate(cmd),
            SubnetSubcommand::Create(c) => c.validate(cmd),
            SubnetSubcommand::Rescue(r) => r.validate(cmd),
        }
    }
}