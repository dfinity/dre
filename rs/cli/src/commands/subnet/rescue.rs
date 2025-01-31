use clap::Args;

use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
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
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Rescue {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = match ctx.runner().await?.subnet_rescue(&self.id, self.keep_nodes.clone(), None).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_with_possible_confirmation(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
