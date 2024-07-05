use analyze::Analyze;
use clap::{Args, Subcommand};
use filter::Filter;
use get::Get;
use list::List;
use pending::Pending;

use super::ExecutableCommand;

mod analyze;
mod filter;
mod get;
mod list;
mod pending;

#[derive(Args, Debug)]
pub struct Proposals {
    #[clap(subcommand)]
    pub subcommand: ProposalsSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ProposalsSubcommands {
    /// Get list of pending proposals
    Pending(Pending),

    /// Get a proposal by ID
    Get(Get),

    /// Print decentralization change for a CHANGE_SUBNET_MEMBERSHIP proposal given its ID
    Analyze(Analyze),

    /// Better proposal filtering
    Filter(Filter),

    /// List proposals
    List(List),
}

impl ExecutableCommand for Proposals {
    fn require_neuron(&self) -> bool {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.require_neuron(),
            ProposalsSubcommands::Get(g) => g.require_neuron(),
            ProposalsSubcommands::Analyze(a) => a.require_neuron(),
            ProposalsSubcommands::Filter(f) => f.require_neuron(),
            ProposalsSubcommands::List(l) => l.require_neuron(),
        }
    }

    fn require_registry(&self) -> bool {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.require_registry(),
            ProposalsSubcommands::Get(g) => g.require_registry(),
            ProposalsSubcommands::Analyze(a) => a.require_registry(),
            ProposalsSubcommands::Filter(f) => f.require_registry(),
            ProposalsSubcommands::List(l) => l.require_registry(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.execute(ctx).await,
            ProposalsSubcommands::Get(g) => g.execute(ctx).await,
            ProposalsSubcommands::Analyze(a) => a.execute(ctx).await,
            ProposalsSubcommands::Filter(f) => f.execute(ctx).await,
            ProposalsSubcommands::List(l) => l.execute(ctx).await,
        }
    }
}
