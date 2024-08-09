use clap::Parser;
use create::Create;
use deploy::Deploy;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;

use super::{impl_executable_command_for_enums, ExecutableCommand, IcAdminRequirement};

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;

#[derive(Parser, Debug)]
pub struct Subnet {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { Deploy, Replace, Resize, Create, Rescue }

impl ExecutableCommand for Subnet {
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
