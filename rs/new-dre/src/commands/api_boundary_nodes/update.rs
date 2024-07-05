use clap::Args;
use ic_types::PrincipalId;

#[derive(Args, Debug)]
pub struct Update {
    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}
