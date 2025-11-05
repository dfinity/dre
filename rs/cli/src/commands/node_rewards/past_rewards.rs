use super::{DateUtc, NodeRewards, ProviderData, fetch_and_aggregate};
use chrono::{DateTime, Datelike};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;

pub async fn run(
    canister_agent: ic_canisters::IcAgentCanisterClient,
    cmd: &NodeRewards,
    month: &str,
) -> anyhow::Result<(chrono::NaiveDate, chrono::NaiveDate, Vec<ProviderData>, Vec<(DateUtc, String, f64)>)> {
    let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
    let governance_client: GovernanceCanisterWrapper = canister_agent.into();

    let gov_list = governance_client.list_node_provider_rewards(None).await?;
    anyhow::ensure!(!gov_list.is_empty(), "No governance rewards snapshots available");
    let target = chrono::NaiveDate::parse_from_str(&(month.to_string() + "-01"), "%Y-%m-%d")?;
    let mut idx_in_month: Option<usize> = None;
    for (i, snap) in gov_list.iter().enumerate() {
        let dt = DateTime::from_timestamp(snap.timestamp as i64, 0).ok_or_else(|| anyhow::anyhow!("Invalid governance timestamp"))?;
        if dt.date_naive().year() == target.year() && dt.date_naive().month() == target.month() {
            idx_in_month = Some(i);
            break;
        }
    }
    let i = idx_in_month.ok_or_else(|| anyhow::anyhow!("No governance snapshot found for {}", month))?;
    let last = &gov_list[i];
    let prev = gov_list
        .get(i + 1)
        .ok_or_else(|| anyhow::anyhow!("Previous governance snapshot not found for {}", month))?;

    let start_day = DateTime::from_timestamp(prev.timestamp as i64, 0).unwrap().date_naive();
    let end_day = DateTime::from_timestamp(last.timestamp as i64, 0)
        .unwrap()
        .date_naive()
        .pred_opt()
        .unwrap();

    let gov_map = last
        .rewards
        .clone()
        .into_iter()
        .map(|r| (r.node_provider.unwrap().id.unwrap(), r.amount_e8s))
        .collect();
    let xdr_permyriad_per_icp: u64 = last.xdr_conversion_rate.clone().unwrap().xdr_permyriad_per_icp.unwrap();

    let (start_day, end_day, provider_data, subnets_fr) =
        fetch_and_aggregate(&node_rewards_client, start_day, end_day, xdr_permyriad_per_icp, gov_map, |daily| {
            cmd.collect_underperforming_nodes(daily)
        })
        .await?;

    Ok((start_day, end_day, provider_data, subnets_fr))
}
