use clap::Args;
use remove::Remove;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

mod remove;

#[derive(Args, Debug)]
pub struct Nodes {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}
impl_executable_command_for_enums! { Remove }

impl ExecutableCommand for Nodes {
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
