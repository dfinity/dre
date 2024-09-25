use clap::Args;
use execute::Execute;
use list::List;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

pub mod execute;
mod list;

#[derive(Args, Debug)]
pub struct Qualify {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { List, Execute }

impl ExecutableCommand for Qualify {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) -> Result<(), clap::Error> {
        self.subcommand.validate(args, cmd)
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }
}
