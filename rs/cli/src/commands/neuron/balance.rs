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
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        match &self.neuron {
            Some(_) => crate::commands::IcAdminRequirement::None,
            None => crate::commands::IcAdminRequirement::Detect,
        }
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let governance = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let neuron_info = governance
            .get_neuron_info(self.neuron.unwrap_or_else(|| ctx.ic_admin().neuron.neuron_id))
            .await?;

        println!("{}", neuron_info.stake_e8s / 10_u64.pow(8));
        Ok(())
    }
}
