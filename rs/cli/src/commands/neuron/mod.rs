use clap::Args;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};
use crate::commands::neuron::balance::Balance;
use crate::commands::neuron::refresh::Refresh;
use crate::commands::neuron::top_up::TopUp;

mod balance;
mod refresh;
mod top_up;

#[derive(Args, Debug)]
pub struct Neuron {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { Balance, TopUp, Refresh }

impl ExecutableCommand for Neuron {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut Command) {
        self.subcommand.validate(args, cmd)
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }
}
