use clap::Args;
use ic_canisters::{governance::GovernanceCanisterWrapper, ledger::LedgerCanisterWrapper};
use itertools::Itertools;

use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};

#[derive(Args, Debug)]
pub struct TopUp {}

impl ExecutableCommand for TopUp {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (neuron, client) = ctx.create_ic_agent_canister_client().await?;
        let governance = GovernanceCanisterWrapper::from(client);
        let full_neuron = governance.get_full_neuron(neuron.neuron_id).await?;
        let ledger = LedgerCanisterWrapper::from(ctx.create_ic_agent_canister_client().await?);
        let account = ledger
            .get_account_id(Some(
                full_neuron
                    .account
                    .try_into()
                    .map_err(|e| anyhow::anyhow!("Expected sub account to be exactly 32 bytes. Full error: {:?}", e))?,
            ))
            .await?;
        let account_hex = account.iter().map(|byte| format!("{:02x}", byte)).join("");

        println!(
            "Please request ICP in the #icp-to-go Slack channel https://dfinity.enterprise.slack.com/archives/C044PCXQJG4 using the following message template.  Be sure to replace 'XX' with the amount of ICP you want:"
        );
        println!(
            "> Hi @icp-dispensers! Can I please get XX ICPs on the account address `{}` for neuron ID {} in order to be able to submit more NNS proposals. Thank you\n",
            account_hex, neuron.neuron_id
        );
        println!("After receiving the ICP, you can check balance of staked ICP and voting power by running `dre neuron balance`");

        Ok(())
    }
}
