use clap::Args;

use crate::commands::node_rewards::compare::PastDistribution;
use crate::commands::node_rewards::ongoing::Ongoing;
use crate::exe::impl_executable_command_for_enums;

mod compare;
mod csv_trait;
mod ongoing;

#[derive(Args, Debug)]
pub struct NodeRewards {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { NodeRewards, PastDistribution, Ongoing }
