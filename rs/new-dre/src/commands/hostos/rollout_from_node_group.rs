use clap::{Args, ValueEnum};

use crate::commands::ExecutableCommand;

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default)]
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

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default)]
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
}

impl ExecutableCommand for RolloutFromNodeGroup {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
