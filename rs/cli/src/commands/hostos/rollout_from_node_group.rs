use std::str::FromStr;

use clap::{Args, ValueEnum};

use crate::{
    auth::{AuthRequirement}, ctx::exe::ExecutableCommand,
    forum::ForumPostKind, submitter::{SubmissionParameters, Submitter},
    operations::hostos_rollout::{NodeGroupUpdate, NumberOfNodes},
};

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default, Hash)]
pub enum NodeOwner {
    Dfinity,
    Others,
    #[default]
    All,
}

impl std::fmt::Display for NodeOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeOwner::Dfinity => write!(f, "DFINITY"),
            NodeOwner::Others => write!(f, "External"),
            NodeOwner::All => write!(f, "DFINITY+External"),
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default, Hash)]
pub enum NodeAssignment {
    Unassigned,
    Assigned,
    #[default]
    All,
}

impl std::fmt::Display for NodeAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeAssignment::Unassigned => write!(f, "Unassigned"),
            NodeAssignment::Assigned => write!(f, "In Subnet"),
            NodeAssignment::All => write!(f, "In Subnet+Unassigned"),
        }
    }
}

#[derive(Args, Debug)]
pub struct RolloutFromNodeGroup {
    /// Version to be rolled out
    #[clap(long)]
    pub version: String,

    /// Filter in for rollout the nodes assigned to a subnet, or unassigned
    #[arg(value_enum)]
    #[clap(long)]
    pub assignment: Option<NodeAssignment>,

    /// Filter in for rollout the DFINITY-owned nodes, or the external-owned nodes
    #[arg(value_enum)]
    #[clap(long)]
    pub owner: Option<NodeOwner>,

    /// Filter in for rollout only the nodes that match the provided list of features
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    /// Filter out (exclude) nodes that match the provided list of features
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    #[clap(
        long,
        help = r#"How many nodes in the group to update with the version specified
supported values are absolute numbers (10) or percentage (10%)"#
    )]
    pub nodes_in_group: String,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for RolloutFromNodeGroup {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let update_group = NodeGroupUpdate::new(self.assignment, self.owner, NumberOfNodes::from_str(&self.nodes_in_group)?);
        let runner = ctx.runner().await?;

        let (nodes_to_update, summary) = match runner
            .hostos_rollout_nodes(update_group, &self.version, &self.only, &self.exclude)
            .await?
        {
            Some(s) => s,
            None => return Ok(()),
        };

        let runner_proposal = runner.hostos_rollout(nodes_to_update, &self.version, Some(summary))?;
        Submitter::from(&self.submission_parameters)
            .propose(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
