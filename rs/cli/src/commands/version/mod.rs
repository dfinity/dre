use clap::Args;

use super::{impl_executable_command_for_enums, ExecutableCommand, IcAdminRequirement};
use crate::commands::version::revise::ReviseElectedVersions;

mod revise;

#[derive(Args, Debug)]
pub struct Version {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { ReviseElectedVersions }

impl ExecutableCommand for Version {
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
