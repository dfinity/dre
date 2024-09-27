use clap::Args;

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Heal {}

impl ExecutableCommand for Heal {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        runner.network_heal(ctx.forum_post_link()).await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
