use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;

use crate::commands::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Balance {
    /// Neuron to query, by default will use the one from configured identity
    #[clap(long)]
    neuron: Option<u64>,
}

impl ExecutableCommand for Balance {
    fn require_auth(&self) -> crate::commands::AuthRequirement {
        match &self.neuron {
            Some(_) => crate::commands::AuthRequirement::Anonymous,
            None => crate::commands::AuthRequirement::Neuron,
        }
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) -> Result<(), clap::Error> {
        Ok(())
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let governance = GovernanceCanisterWrapper::from(ctx.create_ic_agent_canister_client(None)?);
        let neuron_info = governance.get_neuron_info(self.neuron.unwrap_or_else(|| ctx.neuron().neuron_id)).await?;

        println!("{}", neuron_info.stake_e8s / 10_u64.pow(8));
        Ok(())
    }
}
