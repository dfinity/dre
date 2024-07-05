use clap::Args;

#[derive(Args, Debug)]
pub struct Rescue {
    /// Node features or Node IDs to exclude from the replacement
    #[clap(long, num_args(1..))]
    pub keep_nodes: Option<Vec<String>>,
}
