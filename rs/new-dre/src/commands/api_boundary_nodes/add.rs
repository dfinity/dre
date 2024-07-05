use clap::Args;
use ic_types::PrincipalId;

#[derive(Args, Debug)]
pub struct Add {
    /// Node IDs to turn into API BNs
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// guestOS version
    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}
