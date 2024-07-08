use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Get {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Get {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let proposal = client.get_proposal(self.proposal_id).await?;
        println!("{}", serde_json::to_string_pretty(&proposal)?);
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
