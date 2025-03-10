use clap::Args;

use ic_types::PrincipalId;

use crate::exe::args::GlobalArgs;
use crate::{
    auth::AuthRequirement,
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Rescue {
    /// Node features or Node IDs to exclude from the replacement
    #[clap(long, num_args(1..))]
    pub keep_nodes: Option<Vec<String>>,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Rescue {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = match ctx.runner().await?.subnet_rescue(&self.id, self.keep_nodes.clone()).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        Submitter::from(&self.submission_parameters)
            .propose_and_print(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
