use clap::Args;
use ic_types::PrincipalId;

#[derive(Args, Debug)]
pub struct Remove {
    /// Node IDs of API BNs that should be turned into unassigned nodes again
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// Motivation for removing the API BNs
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}
