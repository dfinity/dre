use clap::Args;

use ic_management_types::requests::SubnetResizeRequest;
use ic_types::PrincipalId;

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind},
};

#[derive(Args, Debug)]
pub struct Resize {
    /// Number of nodes to be added
    #[clap(long, default_value = "0")]
    pub add: usize,

    /// Number of nodes to be removed
    #[clap(long, default_value = "0")]
    pub remove: usize,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    /// Force the inclusion of the provided nodes for replacement,
    /// regardless of the decentralization
    #[clap(long, num_args(1..))]
    pub include: Vec<PrincipalId>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: String,

    /// The ID of the subnet.
    #[clap(long, short, alias = "subnet-id")]
    pub id: PrincipalId,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for Resize {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;

        let subnet_manager = ctx.subnet_manager().await?;

        let subnet_change_response = subnet_manager
            .subnet_resize(
                SubnetResizeRequest {
                    subnet: self.id,
                    add: self.add,
                    remove: self.remove,
                    exclude: self.exclude.clone().into(),
                    only: self.only.clone().into(),
                    include: self.include.clone().into(),
                },
                self.motivation.clone(),
                &runner.health_of_nodes().await?,
            )
            .await?;

        let runner_proposal = match runner.propose_subnet_change(subnet_change_response, None).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_run(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
