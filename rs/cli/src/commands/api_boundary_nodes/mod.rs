use add::Add;
use clap::Args;
use remove::Remove;
use update::Update;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

mod add;
mod remove;
mod update;

#[derive(Args, Debug)]
pub struct ApiBoundaryNodes {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { Add, Update, Remove }

impl ExecutableCommand for ApiBoundaryNodes {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        self.subcommand.validate(cmd)
    }
}
