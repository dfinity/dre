use clap::Args;

use super::{impl_executable_command_for_enums, ExecutableCommand, IcAdminRequirement};
use crate::commands::neuron::balance::Balance;

mod balance;
mod top_up;

#[derive(Args, Debug)]
pub struct Neuron {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { Balance }

impl ExecutableCommand for Neuron {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        self.subcommand.require_ic_admin()
    }

    fn validate(&self, cmd: &mut Command) {
        self.subcommand.validate(cmd)
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }
}
