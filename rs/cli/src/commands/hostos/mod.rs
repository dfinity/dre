use clap::Args;
use rollout::Rollout;
use rollout_from_node_group::RolloutFromNodeGroup;

use super::{ExecutableCommand, IcAdminRequirement};

mod rollout;
pub mod rollout_from_node_group;

#[derive(Args, Debug)]
pub struct HostOs {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

super::impl_executable_command_for_enums! { Rollout, RolloutFromNodeGroup }

impl ExecutableCommand for HostOs {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        self.subcommand.require_ic_admin()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        self.subcommand.validate(cmd)
    }
}
