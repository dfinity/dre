use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Pending {}

impl ExecutableCommand for Pending {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_ic_agent_canister_client().await?);
        let proposals = client.get_pending_proposals().await?;
        let proposals = serde_json::to_string(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;

        println!("{}", proposals);

        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
