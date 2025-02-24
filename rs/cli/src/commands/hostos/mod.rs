use clap::Args;
use rollout::Rollout;
use rollout_from_node_group::RolloutFromNodeGroup;

use crate::exe::impl_executable_command_for_enums;

mod rollout;
pub mod rollout_from_node_group;

#[derive(Args, Debug)]
pub struct HostOs {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { HostOs, Rollout, RolloutFromNodeGroup }
