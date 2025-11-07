use super::{fetch_and_aggregate, DateUtc, NodeRewards, ProviderRewards};
use crate::commands::node_rewards::common::NodeRewardsCommand;
use chrono::DateTime;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;

pub struct OngoingRewardsCommand;

impl NodeRewardsCommand for OngoingRewardsCommand {}
impl OngoingRewardsCommand {
    pub async fn run(
        &self,
        canister_agent: ic_canisters::IcAgentCanisterClient,
        cmd: &NodeRewards,
    ) -> anyhow::Result<(chrono::NaiveDate, chrono::NaiveDate, Vec<ProviderRewards>, Vec<(DateUtc, String, f64)>)> {
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

        // Governance map and conversion
        let gov_map = last_rewards
            .rewards
            .clone()
            .into_iter()
            .map(|r| (r.node_provider.unwrap().id.unwrap(), r.amount_e8s))
            .collect();
        let xdr_permyriad_per_icp: u64 = last_rewards.xdr_conversion_rate.clone().unwrap().xdr_permyriad_per_icp.unwrap();

        let (mut nrc_providers_rewards, subnets_failure_rates) = self.fetch_nrc_rewards(&node_rewards_client, start_date, end_date).await?;

        if let Some(provider_filter) = maybe_provider_filter {
            nrc_providers_rewards.retain(|p| {
                let provider_id = p.provider_id.to_string();
                let prefix = provider_id.split('-').next().unwrap();
                provider_id == *provider_filter || prefix == provider_filter
            });
        }

        if let Some(output_path) = maybe_csv_output_path {
            self.generate_csv_files_by_provider(&nrc_providers_rewards, &output_path, &subnets_failure_rates, start_date, end_date)
                .await?;
        } else {
            self.print_rewards_summary_console(&nrc_providers_rewards)?;
        }
    }
}
