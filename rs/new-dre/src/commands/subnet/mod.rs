use clap::{Parser, Subcommand};
use create::Create;
use deploy::Deploy;
use ic_types::PrincipalId;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;

use super::ExecutableCommand;

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;

#[derive(Parser, Debug)]
pub struct SubnetCommand {
    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: Option<PrincipalId>,

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
    fn require_neuron(&self) -> bool {
        match &self.subcommand {
            SubnetSubcommand::Deploy(d) => d.require_neuron(),
            SubnetSubcommand::Replace(r) => r.require_neuron(),
            SubnetSubcommand::Resize(r) => r.require_neuron(),
            SubnetSubcommand::Create(c) => c.require_neuron(),
            SubnetSubcommand::Rescue(r) => r.require_neuron(),
        }
    }

    fn require_registry(&self) -> bool {
        match &self.subcommand {
            SubnetSubcommand::Deploy(d) => d.require_registry(),
            SubnetSubcommand::Replace(r) => r.require_registry(),
            SubnetSubcommand::Resize(r) => r.require_registry(),
            SubnetSubcommand::Create(c) => c.require_registry(),
            SubnetSubcommand::Rescue(r) => r.require_registry(),
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
}
