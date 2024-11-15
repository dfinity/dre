use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Get {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Get {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_ic_agent_canister_client().await?);
        let proposal = client.get_proposal(self.proposal_id).await?;
        println!("{}", serde_json::to_string_pretty(&proposal)?);
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
