use clap::Args;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Heal {}

impl ExecutableCommand for Heal {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        runner.network_heal().await
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
