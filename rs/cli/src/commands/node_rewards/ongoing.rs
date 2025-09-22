use super::{fetch_and_aggregate, NodeRewards, ProviderData};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use rewards_calculation::types::DayUtc;
use rust_decimal::Decimal;

pub async fn run(canister_agent: ic_canisters::IcAgentCanisterClient, cmd: &NodeRewards) -> anyhow::Result<Vec<ProviderData>> {
    let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
    let governance_client: GovernanceCanisterWrapper = canister_agent.into();

    let mut gov_rewards = governance_client.list_node_provider_rewards(None).await?.into_iter();
    let last_rewards = gov_rewards.next().unwrap();

    // Range: from latest governance ts to yesterday
    let today = chrono::Utc::now().date_naive();
    let yesterday = today.pred_opt().ok_or_else(|| anyhow::anyhow!("Cannot compute yesterday"))?;
    let end_ts = yesterday
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid yesterday midnight"))?
        .and_utc()
        .timestamp();
    let start_day = DayUtc::from_secs(last_rewards.timestamp);
    let end_day = DayUtc::from_secs(end_ts as u64);

    // Governance map and conversion
    let gov_map = last_rewards
        .rewards
        .clone()
        .into_iter()
        .map(|r| (r.node_provider.unwrap().id.unwrap(), r.amount_e8s as u64))
        .collect();
    let xdr_permyriad_per_icp: Decimal = last_rewards
        .xdr_conversion_rate
        .clone()
        .unwrap()
        .xdr_permyriad_per_icp
        .clone()
        .unwrap()
        .into();

    fetch_and_aggregate(&node_rewards_client, start_day, end_day, xdr_permyriad_per_icp, gov_map, |daily| {
        cmd.collect_underperforming_nodes(daily)
    })
    .await
}
