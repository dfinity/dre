use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct TopUp {}

impl ExecutableCommand for TopUp {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        crate::commands::IcAdminRequirement::Detect
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let governance = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let full_neuron = governance.get_full_neuron(ctx.ic_admin().neuron.neuron_id).await?;
        let account_hex = full_neuron.account.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();

        Ok(())
    }
}
