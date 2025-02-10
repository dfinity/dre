use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{executor::ForumEnabledProposalExecutor, ForumParameters, ForumPostKind},
};

#[derive(Args, Debug)]
pub struct Rollout {
    /// HostOS version to be rolled out
    #[clap(long)]
    pub version: String,

    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..))]
    pub nodes: Vec<PrincipalId>,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Rollout {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx.runner().await?.hostos_rollout(self.nodes.clone(), &self.version, None)?;
        ForumEnabledProposalExecutor::from_executor_and_mode(&self.forum_parameters, ctx.mode.clone(), ctx.executor().await?)
            .propose(runner_proposal, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
