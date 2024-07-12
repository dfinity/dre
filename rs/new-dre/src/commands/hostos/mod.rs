use clap::{Args, Subcommand};
use rollout::Rollout;
use rollout_from_node_group::RolloutFromNodeGroup;

use super::{ExecutableCommand, IcAdminRequirement};

mod rollout;
pub mod rollout_from_node_group;

#[derive(Args, Debug)]
pub struct HostOsCmd {
    #[clap(subcommand)]
    pub subcommand: HostOsSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum HostOsSubcommands {
    #[clap(about = r#"Roll out an elected HostOS version to the specified list of nodes.
The provided "version" must be already elected. The "nodes" list must
contain the node IDs where the version should be rolled out."#)]
    Rollout(Rollout),

    #[clap(about = r#"Smarter roll out of the elected HostOS version to groups of nodes.
The groups of nodes are created based on assignment to subnets, and on 
the owner of the nodes: DFINITY/other. The provided "version" must be 
already elected."#)]
    RolloutFromNodeGroup(RolloutFromNodeGroup),
}

impl ExecutableCommand for HostOsCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            HostOsSubcommands::Rollout(r) => r.require_ic_admin(),
            HostOsSubcommands::RolloutFromNodeGroup(r) => r.require_ic_admin(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            HostOsSubcommands::Rollout(r) => r.execute(ctx).await,
            HostOsSubcommands::RolloutFromNodeGroup(r) => r.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            HostOsSubcommands::Rollout(r) => r.validate(cmd),
            HostOsSubcommands::RolloutFromNodeGroup(r) => r.validate(cmd),
        }
    }
}
