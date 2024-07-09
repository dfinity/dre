use clap::Args;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Heal {}

impl ExecutableCommand for Heal {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        todo!("Implement once runner is migrated")
        // let runner = ctx.runner();
        // runner.network_heal(true).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
