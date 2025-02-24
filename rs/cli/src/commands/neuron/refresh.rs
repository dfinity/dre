use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};

#[derive(Args, Debug)]
pub struct Refresh {}

impl ExecutableCommand for Refresh {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (neuron, client) = ctx.create_ic_agent_canister_client().await?;
        let governance_canister = GovernanceCanisterWrapper::from(client);

        let resp = governance_canister.refresh_neuron(neuron.neuron_id).await?;
        println!("{:?}", resp);

        Ok(())
    }
}
