use add::Add;
use clap::{Args, Subcommand};
use remove::Remove;
use update::Update;

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
