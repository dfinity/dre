use super::{fetch_and_aggregate, NodeRewards, ProviderData};
use chrono::DateTime;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;

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
    let start_day = DateTime::from_timestamp(last_rewards.timestamp as i64, 0).unwrap().date_naive();
    let end_day = DateTime::from_timestamp(end_ts, 0).unwrap().date_naive();

    // Governance map and conversion
    let gov_map = last_rewards
        .rewards
        .clone()
        .into_iter()
        .map(|r| (r.node_provider.unwrap().id.unwrap(), r.amount_e8s as u64))
        .collect();
    let xdr_permyriad_per_icp: u64 = last_rewards.xdr_conversion_rate.clone().unwrap().xdr_permyriad_per_icp.unwrap();

    fetch_and_aggregate(&node_rewards_client, start_day, end_day, xdr_permyriad_per_icp, gov_map, |daily| {
        cmd.collect_underperforming_nodes(daily)
    })
    .await
}
