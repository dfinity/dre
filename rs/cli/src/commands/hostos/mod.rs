use clap::Args;
use rollout::Rollout;
use rollout_from_node_group::RolloutFromNodeGroup;

use super::{AuthRequirement, ExecutableCommand};

mod rollout;
pub mod rollout_from_node_group;

#[derive(Args, Debug)]
pub struct HostOs {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

super::impl_executable_command_for_enums! { Rollout, RolloutFromNodeGroup }

impl ExecutableCommand for HostOs {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) -> Result<(), clap::Error> {
        self.subcommand.validate(args, cmd)
    }
}
