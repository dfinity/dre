use clap::Args;

#[derive(Args, Debug)]
pub struct Propose {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}
