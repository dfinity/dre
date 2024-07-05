use analyze::Analyze;
use clap::{Args, Subcommand};
use filter::Filter;
use get::Get;
use list::List;
use pending::Pending;

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
