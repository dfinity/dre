use clap::Args;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};
use crate::commands::version::revise::ReviseElectedVersions;

pub(crate) mod revise;

#[derive(Args, Debug)]
pub struct Version {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { ReviseElectedVersions }

impl ExecutableCommand for Version {
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
