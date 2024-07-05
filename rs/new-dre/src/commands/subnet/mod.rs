use clap::{Parser, Subcommand};
use create::Create;
use deploy::Deploy;
use ic_types::PrincipalId;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;

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
