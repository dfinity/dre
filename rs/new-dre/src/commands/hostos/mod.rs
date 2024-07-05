use clap::{Args, Subcommand};
use rollout::Rollout;

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
}
