use clap::{Args, Subcommand};
use remove::Remove;

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
