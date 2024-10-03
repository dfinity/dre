use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Rollout {
    /// HostOS version to be rolled out
    #[clap(long)]
    pub version: String,

    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..))]
    pub nodes: Vec<PrincipalId>,
}

impl ExecutableCommand for Rollout {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        let runner_proposal = runner.hostos_rollout(self.nodes.clone(), &self.version, None, ctx.forum_post_link())?;
        let ic_admin = ctx.ic_admin().await?;
        ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
