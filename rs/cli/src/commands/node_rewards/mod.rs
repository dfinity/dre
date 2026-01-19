use clap::Args;
use ongoing::Ongoing;
use past_rewards::PastRewards;

use crate::exe::impl_executable_command_for_enums;

pub mod common;
mod ongoing;
mod past_rewards;

#[derive(Args, Debug)]
pub struct NodeRewards {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { NodeRewards, Ongoing, PastRewards }
