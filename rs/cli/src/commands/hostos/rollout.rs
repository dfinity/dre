use clap::Args;
use ic_types::PrincipalId;

use crate::{
    auth::AuthRequirement,
    ctx::exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
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
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Rollout {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx.runner().await?.hostos_rollout(self.nodes.clone(), &self.version, None)?;
        Submitter::from(&self.submission_parameters)
            .propose(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
