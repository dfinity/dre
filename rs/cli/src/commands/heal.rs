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
        let proposals = runner.network_heal(ctx.forum_post_link()).await?;
        let ic_admin = ctx.ic_admin().await?;
        let mut errors = vec![];
        for proposal in proposals {
            if let Err(e) = ic_admin.propose_run(proposal.cmd, proposal.opts).await {
                errors.push(e);
            }
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(anyhow::anyhow!("All errors received:\n{:?}", errors)),
        }
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
