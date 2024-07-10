use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Pending {}

impl ExecutableCommand for Pending {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let proposals = client.get_pending_proposals().await?;
        let proposals = serde_json::to_string(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;

        println!("{}", proposals);

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
