use super::{ExecutableCommand, IcAdminRequirement};
use clap::{Args, Subcommand};
use management_canister::ManagementCanister;
use metrics_canister::MetricsCanister;

mod management_canister;
mod metrics_canister;

#[derive(Args, Debug)]
pub struct NodeMetricsCmd {
    #[clap(subcommand)]
    pub subcommand: NodeMetricsCommand,
}

#[derive(Subcommand, Debug)]
pub enum NodeMetricsCommand {
    From(FromNodeMetricsCmd),
}

impl ExecutableCommand for NodeMetricsCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            NodeMetricsCommand::From(c) => c.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            NodeMetricsCommand::From(c) => c.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            NodeMetricsCommand::From(c) => c.validate(cmd),
        }
    }
}


#[derive(Args, Debug)]
pub struct FromNodeMetricsCmd {
    #[clap(subcommand)]
    pub subcommand: FromNodeMetricsCommand,
}

#[derive(Subcommand, Debug)]
pub enum FromNodeMetricsCommand {
    ManagementCanister(ManagementCanister),
    MetricsCanister(MetricsCanister),
}

impl ExecutableCommand for FromNodeMetricsCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            FromNodeMetricsCommand::MetricsCanister(c) => c.require_ic_admin(),
            FromNodeMetricsCommand::ManagementCanister(c) => c.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            FromNodeMetricsCommand::MetricsCanister(c) => c.execute(ctx).await,
            FromNodeMetricsCommand::ManagementCanister(c) => c.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            FromNodeMetricsCommand::MetricsCanister(c) => c.validate(cmd),
            FromNodeMetricsCommand::ManagementCanister(c) => c.validate(cmd),
        }
    }
}
