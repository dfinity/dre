use crate::exe::args::GlobalArgs;
use clap::{error::ErrorKind, Args};
use decentralization::subnets::NodesRemover;

use crate::{
    auth::AuthRequirement,
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Remove {
    /// Skip removal of duplicate or dead nodes
    #[clap(long)]
    pub no_auto: bool,

    /// Remove also degraded nodes; by default only dead (offline) nodes are automatically removed
    #[clap(long)]
    pub remove_degraded: bool,

    /// Specifies the filter used to remove extra nodes
    pub extra_nodes_filter: Vec<String>,

    /// Features or Node IDs to not remove (exclude from the removal)
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Motivation for removing additional nodes
    #[clap(long, aliases = ["summary"])]
    pub motivation: Option<String>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Remove {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx
            .runner()
            .await?
            .remove_nodes(NodesRemover {
                no_auto: self.no_auto,
                remove_degraded: self.remove_degraded,
                extra_nodes_filter: self.extra_nodes_filter.clone(),
                exclude: Some(self.exclude.clone()),
                motivation: self.motivation.clone().unwrap_or_default(),
            })
            .await?;
        Submitter::from(&self.submission_parameters)
            .propose_and_print(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        if self.motivation.is_none() && !self.extra_nodes_filter.is_empty() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument motivation not found")
                .exit()
        }
    }
}
