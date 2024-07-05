use clap::Args;

#[derive(Args, Debug)]
pub struct Deploy {
    /// Version to propose for the subnet
    #[clap(long, short)]
    pub version: String,
}
