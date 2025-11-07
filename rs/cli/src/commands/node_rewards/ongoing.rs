use crate::commands::node_rewards::NodeRewards;
use crate::commands::node_rewards::common::NodeRewardsCommand;
use chrono::DateTime;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;

pub struct OngoingRewardsCommand;

impl NodeRewardsCommand for OngoingRewardsCommand {}

impl OngoingRewardsCommand {
    pub async fn run(
        canister_agent: ic_canisters::IcAgentCanisterClient,
        _cmd: &NodeRewards,
        csv_detailed_output_path: Option<&str>,
        provider_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        let mut gov_rewards = governance_client.list_node_provider_rewards(None).await?.into_iter();
        let last_rewards = gov_rewards.next().unwrap();

        // Range: from latest governance ts to yesterday
        let today = chrono::Utc::now().date_naive();
        let yesterday = today.pred_opt().unwrap();
        let end_ts = yesterday
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid yesterday midnight"))?
            .and_utc()
            .timestamp();
        let start_day = DateTime::from_timestamp(last_rewards.timestamp as i64, 0).unwrap().date_naive();
        let end_day = DateTime::from_timestamp(end_ts, 0).unwrap().date_naive();

        let command = OngoingRewardsCommand;
        let (mut nrc_providers_rewards, subnets_failure_rates) = command.fetch_nrc_rewards(&node_rewards_client, start_day, end_day).await?;

        if let Some(provider_filter) = provider_id {
            nrc_providers_rewards.retain(|p| {
                let provider_id_str = p.provider_id.to_string();
                let prefix = OngoingRewardsCommand::get_provider_prefix(&provider_id_str);
                provider_id_str == provider_filter || prefix == provider_filter
            });
        }

        if let Some(output_path) = csv_detailed_output_path {
            command
                .generate_csv_files_by_provider(&nrc_providers_rewards, output_path, &subnets_failure_rates, start_day, end_day)
                .await?;
        } else {
            command.print_rewards_summary_console(&nrc_providers_rewards)?;
        }

        Ok(())
    }
}
