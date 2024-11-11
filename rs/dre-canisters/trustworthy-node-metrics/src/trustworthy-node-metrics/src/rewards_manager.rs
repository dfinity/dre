use candid::Principal;
use ic_base_types::PrincipalId;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance_api::pb::v1::MonthlyNodeProviderRewards;
use ic_protobuf::registry::node_rewards::{v2::NodeRewardRate, v2::NodeRewardsTable};
use ic_registry_keys::NODE_REWARDS_TABLE_KEY;
use itertools::Itertools;
use node_provider_rewards_lib::{
    v1_rewards::{assigned_nodes_multiplier, calculate_rewards},
    v1_types::{AHashMap, DailyNodeMetrics as NPRDailyNodeMetrics, Node},
};
use num_traits::ToPrimitive;
use trustworthy_node_metrics_types::types::{DailyNodeMetrics, NodeProviderRewards, NodeRewardsMultiplier};

use crate::{chrono_utils::DateTimeRange, stable_memory};

fn get_daily_metrics(node_ids: Vec<Principal>, rewarding_period: DateTimeRange) -> AHashMap<Principal, Vec<DailyNodeMetrics>> {
    let mut daily_metrics: AHashMap<Principal, Vec<DailyNodeMetrics>> = AHashMap::default();
    let nodes_metrics = stable_memory::get_metrics_range(
        rewarding_period.start_timestamp_nanos(),
        Some(rewarding_period.end_timestamp_nanos()),
        Some(&node_ids),
    );

    for node_id in node_ids {
        daily_metrics.entry(node_id).or_default();
    }

    for ((ts, node_id), node_metrics_value) in nodes_metrics {
        let daily_node_metrics = DailyNodeMetrics::new(
            ts,
            node_metrics_value.subnet_assigned,
            node_metrics_value.num_blocks_proposed,
            node_metrics_value.num_blocks_failed,
        );

        daily_metrics.entry(node_id).or_default().push(daily_node_metrics);
    }
    daily_metrics
}

pub fn node_rewards_multiplier(node_ids: Vec<Principal>, rewarding_period: DateTimeRange) -> Vec<NodeRewardsMultiplier> {
    let total_days = rewarding_period.days_between();
    let daily_metrics = get_daily_metrics(node_ids, rewarding_period);
    let rewards_table = stable_memory::get_node_rewards_table();

    daily_metrics
        .into_iter()
        .map(|(node_id, daily_node_metrics)| {
            let npr_daily_metrics = daily_node_metrics
                .iter()
                .map(|metrics| NPRDailyNodeMetrics {
                    num_blocks_proposed: metrics.num_blocks_proposed,
                    num_blocks_failed: metrics.num_blocks_failed,
                })
                .collect_vec();

            let (rewards_multiplier, rewards_multiplier_stats) = assigned_nodes_multiplier(&npr_daily_metrics, total_days);
            let node_metadata = stable_memory::get_node_metadata(&node_id).expect("Node should have one node provider");

            let node_rate = match rewards_table.get_rate(&node_metadata.region, &node_metadata.node_type) {
                Some(rate) => rate,
                None => {
                    println!(
                        "The Node Rewards Table does not have an entry for \
                             node type '{}' within region '{}' or parent region, defaulting to 1 xdr per month per node",
                        node_metadata.region, node_metadata.node_type
                    );
                    NodeRewardRate {
                        xdr_permyriad_per_node_per_month: 1,
                        reward_coefficient_percent: Some(100),
                    }
                }
            };

            NodeRewardsMultiplier {
                node_id,
                daily_node_metrics,
                node_rate,
                rewards_multiplier: rewards_multiplier.to_f64().unwrap(),
                rewards_multiplier_stats,
            }
        })
        .collect_vec()
}

pub fn node_provider_rewards(node_provider_id: Principal, rewarding_period: DateTimeRange) -> NodeProviderRewards {
    let total_days = rewarding_period.days_between();
    let rewards_table = stable_memory::get_node_rewards_table();
    let np_id = PrincipalId::from(node_provider_id);

    let latest_np_rewards = stable_memory::get_latest_node_providers_rewards();
    let nodes_in_period = stable_memory::get_node_principals(&node_provider_id)
        .into_iter()
        .map(|node| {
            let meta = stable_memory::get_node_metadata(&node).unwrap();
            Node {
                node_id: PrincipalId::from(node),
                node_provider_id: np_id,
                region: meta.region,
                node_type: meta.node_type,
            }
        })
        .collect_vec();

    let node_metrics_in_period = get_daily_metrics(nodes_in_period.iter().map(|node| node.node_id.0).collect(), rewarding_period)
        .into_iter()
        .map(|(np, metrics)| {
            (
                PrincipalId::from(np),
                metrics
                    .into_iter()
                    .map(|m| NPRDailyNodeMetrics {
                        num_blocks_proposed: m.num_blocks_proposed,
                        num_blocks_failed: m.num_blocks_failed,
                    })
                    .collect_vec(),
            )
        })
        .collect();

    let rewards = calculate_rewards(total_days, &rewards_table, &nodes_in_period, &node_metrics_in_period);

    let rewards_per_np = rewards.rewards_per_node_provider.get(&np_id).unwrap();

    let rewards_multipliers_stats = rewards_per_np.1.clone().into_iter().map(|(_, stats)| stats).collect_vec();
    let rewards_ammount = &rewards.rewards_per_node_provider.get(&np_id).unwrap().0;

    let rewards_xdr_old = latest_np_rewards
        .rewards
        .into_iter()
        .filter_map(|np_rewards| {
            if let Some(node_provider) = np_rewards.node_provider {
                if let Some(id) = node_provider.id {
                    if id.0 == node_provider_id {
                        return Some(np_rewards.amount_e8s);
                    }
                }
            }
            None
        })
        .next();

    NodeProviderRewards {
        node_provider_id,
        rewards_xdr_permyriad: rewards_ammount.xdr_permyriad,
        rewards_xdr_permyriad_no_reduction: rewards_ammount.xdr_permyriad_no_reduction,
        computation_log: rewards.rewards_log_per_node_provider.get(&np_id).unwrap().get_log(),
        rewards_xdr_old,
        ts_distribution: latest_np_rewards.timestamp,
        xdr_conversion_rate: latest_np_rewards.xdr_conversion_rate.and_then(|rate| rate.xdr_permyriad_per_icp),
        rewards_multipliers_stats,
    }
}

/// Update node rewards table
pub async fn update_node_rewards_table() -> anyhow::Result<()> {
    let (rewards_table, _): (NodeRewardsTable, _) = ic_nns_common::registry::get_value(NODE_REWARDS_TABLE_KEY.as_bytes(), None).await?;
    for (region, rewards_rates) in rewards_table.table {
        stable_memory::insert_rewards_rates(region, rewards_rates)
    }

    Ok(())
}

/// Update recent node providers rewards
pub async fn update_recent_provider_rewards() -> anyhow::Result<()> {
    let (maybe_monthly_rewards,): (Option<MonthlyNodeProviderRewards>,) = ic_cdk::api::call::call(
        Principal::from(GOVERNANCE_CANISTER_ID),
        "get_most_recent_monthly_node_provider_rewards",
        (),
    )
    .await
    .map_err(|(code, msg)| {
        anyhow::anyhow!(
            "Error when calling get_most_recent_monthly_node_provider_rewards:\n Code:{:?}\nMsg:{}",
            code,
            msg
        )
    })?;

    if let Some(monthly_rewards) = maybe_monthly_rewards {
        let latest_np_rewards = stable_memory::get_latest_node_providers_rewards();

        if latest_np_rewards.timestamp < monthly_rewards.timestamp {
            stable_memory::insert_node_provider_rewards(monthly_rewards.timestamp, monthly_rewards)
        }
    }

    Ok(())
}
