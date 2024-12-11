use candid::Principal;
use ic_base_types::PrincipalId;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance_api::pb::v1::MonthlyNodeProviderRewards;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardsTable};
use ic_registry_keys::NODE_REWARDS_TABLE_KEY;
use ic_registry_node_provider_rewards::v1_rewards::calculate_rewards as calculate_rewards_with_subnets;
use ic_registry_node_provider_rewards::v1_types::RewardableNode;
use itertools::Itertools;
use node_provider_rewards_lib::{
    v1_rewards::assigned_nodes_multiplier,
    v1_types::DailyNodeMetrics as NPRDailyNodeMetrics,
};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use trustworthy_node_metrics_types::types::{DailyNodeMetrics, NodeProviderRewards, NodeRewardsMultiplier, RewardsWithLogs};

use crate::stable_memory::REWARDS_BY_NODE_PROVIDER;
use crate::{chrono_utils::DateTimeRange, registry_querier::RegistryQuerier, stable_memory};

fn get_daily_metrics(node_ids: Vec<Principal>, rewarding_period: DateTimeRange) -> HashMap<Principal, Vec<DailyNodeMetrics>> {
    let mut daily_metrics: HashMap<Principal, Vec<DailyNodeMetrics>> = HashMap::default();
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

pub fn node_provider_rewards(node_provider_id: Principal, rewarding_period: DateTimeRange, registry_querier: RegistryQuerier) -> NodeProviderRewards {
    let latest_np_rewards = stable_memory::get_latest_node_providers_rewards();
    let latest_rewards_ts = latest_np_rewards.timestamp * 1_000_000_000;
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

    let rewards= REWARDS_BY_NODE_PROVIDER.with_borrow(|rewards_by_node_provider| {
        rewards_by_node_provider
            .get(&(latest_rewards_ts, node_provider_id))
            .expect("Node Provider rewards should be stored in memory")
    });

    NodeProviderRewards  {
        node_provider_id,
        rewards_xdr_permyriad: rewards.xdr_permyriad,
        rewards_xdr_permyriad_no_reduction: rewards.xdr_permyriad_no_reduction,
        computation_log: rewards.logs,
        rewards_xdr_old,
        ts_distribution: latest_np_rewards.timestamp,
        xdr_conversion_rate: latest_np_rewards.xdr_conversion_rate.and_then(|rate| rate.xdr_permyriad_per_icp),
    }
}

pub async fn store_node_provider_rewards_with_subnets(registry_querier: RegistryQuerier) -> anyhow::Result<()> {

    // REWARDS_BY_NODE_PROVIDER.with_borrow_mut(|rewards_by_node_provider| rewards_by_node_provider.clear_new());

    let rewards_table: NodeRewardsTable = stable_memory::get_node_rewards_table();
    let timestamp_ns_last_prod_rewards = stable_memory::get_latest_node_providers_rewards().timestamp * 1_000_000_000;
    let rewarding_period = DateTimeRange::from_end_ts(timestamp_ns_last_prod_rewards);

    let last_rewards_end_ts = REWARDS_BY_NODE_PROVIDER.with_borrow(|rewards_by_node_provider| {
        rewards_by_node_provider.last_key_value()
            .map(|((ts, _), _)| ts)
    });

    if timestamp_ns_last_prod_rewards == last_rewards_end_ts.unwrap_or(0) {
        ic_cdk::println!("Node Provider rewards already stored for last reward period");
        return Ok(());
    }

    let total_days = rewarding_period.days_between();

    let nodes_in_period = registry_querier
        .nodes_in_period(&rewarding_period)
        .into_iter()
        .map(|node| {
            RewardableNode {
                node_id: node.node_id,
                node_provider_id: node.node_provider_id,
                region: node.region,
                node_type: node.node_type
            }
        })
        .collect_vec();

    let subnets = crate::metrics_manager::fetch_subnets().await?;
    let subnet_metrics: HashMap<PrincipalId, Vec<NodeMetricsHistoryResponse>> = crate::metrics_manager::fetch_metrics(subnets, rewarding_period.start_timestamp_nanos())
        .await?
        .into_iter()
        .map(|(subnet_id, metrics)| {
            let metrics_filtered = metrics
                .into_iter()
                .filter(|m| m.timestamp_nanos <= rewarding_period.end_timestamp_nanos())
                .collect_vec();

            (subnet_id, metrics_filtered)
        })
        .collect();


    ic_cdk::println!("Subnets metrics: {:?}", subnet_metrics);


    let rewards = calculate_rewards_with_subnets(
        total_days,
        &rewards_table,
        subnet_metrics,
        &nodes_in_period
    );

    REWARDS_BY_NODE_PROVIDER.with_borrow_mut(|rewards_by_node_provider| {
        ic_cdk::println!("Storing {} rewards", rewards.rewards_per_node_provider.len());
        for (np_id, rewards_given) in rewards.rewards_per_node_provider {
            let rewards_stored = RewardsWithLogs{
                xdr_permyriad: rewards_given.xdr_permyriad,
                xdr_permyriad_no_reduction: rewards_given.xdr_permyriad_no_reduction,
                logs: rewards.rewards_log_per_node_provider.get(&np_id).unwrap().get_log(),
            };
            rewards_by_node_provider.insert((timestamp_ns_last_prod_rewards, np_id.0), rewards_stored);
        }
    });

    Ok(())
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
