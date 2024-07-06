use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::{ExecutableCommand, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Pending {}

impl ExecutableCommand for Pending {
    fn require_neuron(&self) -> bool {
        false
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let proposals = client.get_pending_proposals().await?;
        let proposals = serde_json::to_string(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;

        println!("{}", proposals);

        Ok(())
    }
}
