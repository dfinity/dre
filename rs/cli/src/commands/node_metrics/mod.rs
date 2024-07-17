use super::{ExecutableCommand, IcAdminRequirement};
use clap::{Args, Subcommand};
use from_node_metrics::FromNodeMetrics;
use from_subnet_management::FromSubnetManagement;

mod from_node_metrics;
mod from_subnet_management;

#[derive(Args, Debug)]
pub struct NodeMetricsCmd {
    #[clap(subcommand)]
    pub subcommand: NodeMetricsCommand,
}

#[derive(Subcommand, Debug)]
pub enum NodeMetricsCommand {
    FromSubnetManagementCanister(FromSubnetManagement),
    FromNodeMetricsCanister(FromNodeMetrics),
}

impl ExecutableCommand for NodeMetricsCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            NodeMetricsCommand::FromNodeMetricsCanister(c) => c.require_ic_admin(),
            NodeMetricsCommand::FromSubnetManagementCanister(c) => c.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            NodeMetricsCommand::FromNodeMetricsCanister(c) => c.execute(ctx).await,
            NodeMetricsCommand::FromSubnetManagementCanister(c) => c.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            NodeMetricsCommand::FromNodeMetricsCanister(c) => c.validate(cmd),
            NodeMetricsCommand::FromSubnetManagementCanister(c) => c.validate(cmd),
        }
    }
}
