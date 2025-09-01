use clap::Args;

use ic_management_types::requests::SubnetResizeRequest;
use ic_types::PrincipalId;

use crate::exe::args::GlobalArgs;
use crate::{
    auth::AuthRequirement,
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Resize {
    /// Number of nodes to be added
    #[clap(long, default_value = "0")]
    pub add_count: usize,

    /// Number of nodes to be removed
    #[clap(long, default_value = "0")]
    pub remove_count: usize,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    /// Add specific nodes to the subnet. Fails if a node is unavailable/unhealthy.
    #[clap(long = "add-nodes", num_args(1..), visible_aliases = &["add", "add-node", "add-node-id", "add-node-ids"])]
    pub add_nodes: Vec<PrincipalId>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: String,

    /// The ID of the subnet.
    #[clap(long, short, alias = "subnet-id")]
    pub id: PrincipalId,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Resize {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;

        let subnet_change_response = ctx
            .subnet_manager()
            .await?
            .subnet_resize(
                SubnetResizeRequest {
                    subnet: self.id,
                    add: self.add_count,
                    remove: self.remove_count,
                    exclude: self.exclude.clone().into(),
                    only: self.only.clone().into(),
                    add_nodes: self.add_nodes.clone().into(),
                },
                self.motivation.clone(),
                &runner.health_of_nodes().await?,
            )
            .await?;

        let runner_proposal = match runner.propose_subnet_change(&subnet_change_response, false).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        Submitter::from(&self.submission_parameters)
            .propose_and_print(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
