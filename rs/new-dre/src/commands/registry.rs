use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub struct Registry {
    /// Version to dump. If value is less than 0 will dump the latest version
    #[clap(long, default_value = "-1")]
    pub version: i64,

    /// Output file (default is stdout)
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Output only information related to the node operator records with incorrect rewards
    #[clap(long)]
    pub incorrect_rewards: bool,

    /// Optional path to cached registry, can be used to inspect an arbitrary path
    #[clap(long, env = "LOCAL_REGISTRY_PATH")]
    pub local_registry_path: Option<PathBuf>,
}
