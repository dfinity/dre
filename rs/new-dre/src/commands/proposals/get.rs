use clap::Args;

#[derive(Args, Debug)]
pub struct Get {
    /// Proposal ID
    proposal_id: u64,
}
