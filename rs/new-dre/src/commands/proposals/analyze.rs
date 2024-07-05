use clap::Args;

#[derive(Args, Debug)]
pub struct Analyze {
    /// Proposal ID
    proposal_id: u64,
}
