use crate::commands::IcAdminRequirement;
use clap::Args;
use execute::Execute;
use list::List;

use super::{impl_executable_command_for_enums, ExecutableCommand};

pub mod execute;
mod list;

#[derive(Args, Debug)]
pub struct Qualify {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { List, Execute }

impl ExecutableCommand for Qualify {
    fn require_ic_admin(&self) -> super::IcAdminRequirement {
        self.subcommand.require_ic_admin()
    }

    fn validate(&self, cmd: &mut clap::Command) {
        self.subcommand.validate(cmd)
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }
}
