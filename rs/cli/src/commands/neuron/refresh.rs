use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Refresh {}

impl ExecutableCommand for Refresh {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        crate::commands::IcAdminRequirement::Detect
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let governance_canister = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);

        let resp = governance_canister.refresh_neuron(ctx.ic_admin().neuron.neuron_id).await?;
        println!("{:?}", resp);

        Ok(())
    }
}
