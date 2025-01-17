use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Balance {}

impl ExecutableCommand for Balance {
    fn require_auth(&self) -> crate::commands::AuthRequirement {
        crate::commands::AuthRequirement::Neuron
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (neuron, client) = ctx.create_ic_agent_canister_client().await?;
        let governance = GovernanceCanisterWrapper::from(client);
        let neuron_id = ctx.neuron_id().unwrap_or(neuron.neuron_id);
        let neuron_info = governance.get_neuron_info(neuron_id).await?;

        println!("{}", neuron_info.stake_e8s / 10_u64.pow(8));
        Ok(())
    }
}
