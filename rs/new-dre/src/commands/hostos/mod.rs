use clap::{Args, Subcommand};
use rollout::Rollout;
use rollout_from_node_group::RolloutFromNodeGroup;

mod rollout;
mod rollout_from_node_group;

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
