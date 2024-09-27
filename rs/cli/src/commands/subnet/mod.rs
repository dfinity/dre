use clap::Parser;
use create::Create;
use deploy::Deploy;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;
use whatif::WhatifDecentralization;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;
mod whatif;

#[derive(Parser, Debug)]
pub struct Subnet {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { WhatifDecentralization, Deploy, Replace, Resize, Create, Rescue }

impl ExecutableCommand for Subnet {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) {
        self.subcommand.validate(args, cmd)
    }
}
