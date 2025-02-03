use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
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
        let runner_proposal = ctx.runner().await?.hostos_rollout(self.nodes.clone(), &self.version, None, None)?;
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_with_possible_confirmation(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
