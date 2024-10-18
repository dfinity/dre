use std::collections::{BTreeMap, HashSet};

use candid::Principal;
use dfn_core::api::PrincipalId;
use futures::FutureExt;
use ic_base_types::NodeId;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use prost::Message;
use ic_registry_keys::{
    make_data_center_record_key, make_node_operator_record_key, make_node_record_key, make_subnet_list_record_key, NODE_REWARDS_TABLE_KEY,
};

use crate::chrono_utils::DateTimeRange;
use crate::stable_memory::{self, RegionNodeTypeCategory};
use crate::types::{DailyNodeMetrics, NodeMetadata, RewardableNodes};
use crate::types::{NodeMetricsGrouped, NodeMetricsStored, NodeMetricsStoredKey};
use crate::types::{SubnetNodeMetricsHistory, TimestampNanos};
use itertools::Itertools;

/// Node metrics storable
///
/// Computes daily proposed/failed blocks from a vector of node metrics
fn node_metrics_storable(
    node_id: PrincipalId,
    node_metrics_grouped: Vec<NodeMetricsGrouped>,
    initial_proposed_total: u64,
    initial_failed_total: u64,
) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    let mut metrics_ordered = node_metrics_grouped;
    metrics_ordered.sort_by_key(|(ts, _, _)| *ts);

    let principal = node_id.0;
    let mut node_metrics_storable = Vec::new();

    let mut previous_proposed_total = initial_proposed_total;
    let mut previous_failed_total = initial_failed_total;

    for (ts, subnet_assigned, metrics) in metrics_ordered {
        let key = (ts, principal);
        let current_proposed_total = metrics.num_blocks_proposed_total;
        let current_failed_total = metrics.num_block_failures_total;

        let (daily_proposed, daily_failed) = calculate_daily_metrics(
            previous_proposed_total,
            previous_failed_total,
            metrics.num_blocks_proposed_total,
            metrics.num_block_failures_total,
        );

        let node_metrics_stored = NodeMetricsStored {
            subnet_assigned: subnet_assigned.0,
            num_blocks_proposed_total: current_proposed_total,
            num_blocks_failures_total: current_failed_total,
            num_blocks_proposed: daily_proposed,
            num_blocks_failed: daily_failed,
        };

        node_metrics_storable.push((key, node_metrics_stored));

        previous_proposed_total = current_proposed_total;
        previous_failed_total = current_failed_total;
    }

    node_metrics_storable
}

/// Fetch metrics
///
/// Calls to the node_metrics_history endpoint of the management canister for all the subnets
/// to get updated metrics since refresh_ts.
async fn fetch_metrics(subnets: Vec<PrincipalId>, refresh_ts: TimestampNanos) -> anyhow::Result<Vec<SubnetNodeMetricsHistory>> {
    let mut subnets_node_metrics = Vec::new();

    for subnet_id in subnets {
        let contract = NodeMetricsHistoryArgs {
            subnet_id,
            start_at_timestamp_nanos: refresh_ts,
        };

        let node_metrics = ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            candid::Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128,
        )
        .map(move |result| {
            result
                .map_err(|(code, msg)| {
                    anyhow::anyhow!(
                        "Error when calling management canister for subnet {}:\n Code:{:?}\nMsg:{}",
                        subnet_id,
                        code,
                        msg
                    )
                })
                .map(|(node_metrics,)| (subnet_id, node_metrics))
        });

        subnets_node_metrics.push(node_metrics);
    }

    let updated_metrics = futures::future::try_join_all(subnets_node_metrics).await?;

    for (subnet, node_metrics) in &updated_metrics {
        ic_cdk::println!("Fetched {} new metrics for subnet: {}", node_metrics.len(), subnet);
    }

    Ok(updated_metrics)
}

// Calculates the daily proposed and failed blocks
fn calculate_daily_metrics(last_proposed_total: u64, last_failed_total: u64, current_proposed_total: u64, current_failed_total: u64) -> (u64, u64) {
    if last_failed_total > current_failed_total || last_proposed_total > current_proposed_total {
        // This is the case when node gets redeploied
        (current_proposed_total, current_failed_total)
    } else {
        (current_proposed_total - last_proposed_total, current_failed_total - last_failed_total)
    }
}

fn grouped_by_node(subnet_metrics: Vec<(PrincipalId, Vec<NodeMetricsHistoryResponse>)>) -> BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> {
    let mut grouped_by_node: BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> = BTreeMap::new();

    for (subnet_id, history) in subnet_metrics {
        for history_response in history {
            for metrics in history_response.node_metrics {
                grouped_by_node
                    .entry(metrics.node_id)
                    .or_default()
                    .push((history_response.timestamp_nanos, subnet_id, metrics));
            }
        }
    }
    grouped_by_node
}

/// Fetch subnets
///
/// Fetch subnets from the registry canister
async fn fetch_subnets() -> anyhow::Result<Vec<PrincipalId>> {
    let (registry_subnets, _): (SubnetListRecord, _) = ic_nns_common::registry::get_value(make_subnet_list_record_key().as_bytes(), None).await?;
    let subnets = registry_subnets
        .subnets
        .into_iter()
        .map(|subnet_id: Vec<u8>| PrincipalId::try_from(subnet_id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(subnets)
}

/// Update node metrics
pub async fn sync_node_metrics() -> anyhow::Result<()> {
    let subnets = fetch_subnets().await?;
    let latest_ts = stable_memory::latest_ts().unwrap_or_default();
    let refresh_ts = latest_ts + 1;

    ic_cdk::println!(
        "Updating node metrics for {} subnets: Latest timestamp persisted: {}  Refreshing metrics from timestamp {}",
        subnets.len(),
        latest_ts,
        refresh_ts
    );
    let subnet_metrics: Vec<(PrincipalId, Vec<NodeMetricsHistoryResponse>)> = fetch_metrics(subnets, refresh_ts).await?;
    let metrics_by_node: BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> = grouped_by_node(subnet_metrics);
    let principals = metrics_by_node.keys().map(|p| p.0).collect_vec();
    let latest_metrics = stable_memory::latest_metrics(&principals);

    for (node_id, node_metrics_grouped) in metrics_by_node {
        let (initial_proposed_total, initial_failed_total) = latest_metrics
            .get(&node_id.0)
            .map(|metrics| (metrics.num_blocks_proposed_total, metrics.num_blocks_failures_total))
            .unwrap_or((0, 0));
        let node_metrics_storable = node_metrics_storable(node_id, node_metrics_grouped, initial_proposed_total, initial_failed_total);

        for (key, node_metrics) in node_metrics_storable {
            stable_memory::insert_node_metrics(key, node_metrics)
        }
    }

    Ok(())
}

async fn get_most_recent_registry_value<T: Message + Default>(
    key: &[u8],
    latest_registry_version: u64,
) -> anyhow::Result<T> {
    for registry_version in (0..=latest_registry_version).rev() {
        ic_cdk::println!("Trying registry version: {}", registry_version);

        match ic_nns_common::registry::get_value::<T>(
            key,
            Some(registry_version)
        ).await {
            Ok((node_record, _)) => {
                return Ok(node_record);
            }
            Err(e) => {
                ic_cdk::println!("{}", e);
            }
        }
    }
    Err(anyhow::anyhow!("Error getting key in any registry version"))
}


pub async fn get_node_operators_rewardables(rewarding_period: &DateTimeRange) -> anyhow::Result<BTreeMap<PrincipalId, RewardableNodes>> {
    // let (registry_node_operators, _): (NodeOperatorListRecord, _) =
    //     ic_nns_common::registry::get_value("node_operator_list".as_bytes(), None).await?;
    // let node_operators = registry_node_operators
    //     .node_operators
    //     .into_iter()
    //     .map(|node_operator_id: Vec<u8>| PrincipalId::try_from(node_operator_id))
    //     .collect::<Result<Vec<_>, _>>()?;

    let node_ids: Vec<Principal> = get_daily_metrics(rewarding_period).keys().cloned().collect();

    let mut node_operators = Vec::new();
    let latest_registry_version = ic_nns_common::registry::get_latest_version().await;

    for (index, node_id) in node_ids.iter().take(10).enumerate() {
        ic_cdk::println!("Fetching NodeRecord from registry canister for node: {} {}", node_id, index);

        let node_record = get_most_recent_registry_value::<NodeRecord>(
            make_node_record_key(NodeId::from(PrincipalId::from(*node_id))).as_bytes(),
            latest_registry_version
        ).await?;

        let node_operator_id: PrincipalId = match node_record.node_operator_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                ic_cdk::println!("Error converting node operator ID for {}: {:?}", node_id, e);
                continue;
            }
        };

        node_operators.push(node_operator_id);
    }

    let mut rewardables = BTreeMap::new();

    for node_operator_id in node_operators {

        ic_cdk::println!("Fetching NodeOperatorRecord from registry canister for operator: {}", node_operator_id);
        let node_operator_record = get_most_recent_registry_value::<NodeOperatorRecord>(
            make_node_operator_record_key(node_operator_id).as_bytes(),
            latest_registry_version
        ).await?;

        let dc_id = node_operator_record.dc_id;

        let node_provider_id: PrincipalId = match node_operator_record.node_provider_principal_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                ic_cdk::println!("Error converting node provider ID {:?}", e);
                continue;
            }
        };

        ic_cdk::println!("Fetching DataCenterRecord from registry canister for dc: {}", dc_id);
        let data_center_record = get_most_recent_registry_value::<DataCenterRecord>(
            make_data_center_record_key(&dc_id).as_bytes(),
            latest_registry_version
        ).await?;

        let region = data_center_record.region;

        rewardables.insert(
            node_operator_id,
            RewardableNodes {
                node_provider_id: node_provider_id.0,
                region: region.clone(),
                rewardables: node_operator_record.rewardable_nodes,
            },
        );
        ic_cdk::println!("Performance counter after node operator fetching: {}", ic_cdk::api::performance_counter(1));
    }

    Ok(rewardables)
}

fn get_daily_metrics(rewarding_period: &DateTimeRange) -> BTreeMap<Principal, Vec<DailyNodeMetrics>> {
    let mut daily_metrics: BTreeMap<Principal, Vec<DailyNodeMetrics>> = BTreeMap::new();
    let nodes_metrics = stable_memory::get_metrics_range(
        rewarding_period.start_timestamp_nanos(),
        Some(rewarding_period.end_timestamp_nanos()),
        None,
    );

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

pub async fn get_assigned_nodes_performance(
    mut rewardable_nodes: BTreeMap<PrincipalId, RewardableNodes>,
    rewarding_period: &DateTimeRange,
) -> anyhow::Result<BTreeMap<Principal, (NodeMetadata, Vec<DailyNodeMetrics>)>> {
    let mut nodes_metadata: BTreeMap<Principal, (NodeMetadata, Vec<DailyNodeMetrics>)> = BTreeMap::new();
    let daily_metrics: BTreeMap<Principal, Vec<DailyNodeMetrics>> = get_daily_metrics(rewarding_period).into_iter().take(n).collect();
    let latest_registry_version = ic_nns_common::registry::get_latest_version().await;

    for (node_principal, daily_metrics) in daily_metrics {
        let node_id = PrincipalId::from(node_principal);

        ic_cdk::println!("Fetching NodeRecord from registry canister for node: {}", node_principal);
        let node_record = get_most_recent_registry_value::<NodeRecord>(
            make_node_record_key(NodeId::from(node_id)).as_bytes(),
            latest_registry_version
        ).await?;


        let node_operator_id: PrincipalId = match node_record.node_operator_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                ic_cdk::println!("Error converting node operator ID for {}: {:?}", node_principal, e);
                continue;
            }
        };

        let rewardables = rewardable_nodes.get_mut(&node_operator_id).unwrap();

        let node_type = if rewardables.rewardables.is_empty() {
            "unknown:no_rewardable_nodes_found".to_string()
        } else {
            // Find the first non-zero rewardable node type, or "unknown" if none are found
            let (k, mut v) = loop {
                let (k, v) = match rewardables.rewardables.pop_first() {
                    Some(kv) => kv,
                    None => break ("unknown:rewardable_nodes_used_up".to_string(), 0),
                };
                if v != 0 {
                    break (k, v);
                }
            };
            v = v.saturating_sub(1);
            // Insert back if not zero
            if v != 0 {
                rewardables.rewardables.insert(k.clone(), v);
            }
            k
        };

        nodes_metadata.insert(
            node_principal,
            (NodeMetadata {
                node_provider_id: rewardables.node_provider_id,
                region: rewardables.region.clone(),
                node_type,
            },
            daily_metrics)
        );
    }

    Ok(nodes_metadata)
}

pub(crate) fn get_node_providers_rewardables(
    node_operators_rewardables: BTreeMap<PrincipalId, RewardableNodes>,
) -> BTreeMap<(Principal, RegionNodeTypeCategory), u32> {
    let mut node_providers_rewardables: BTreeMap<(Principal, RegionNodeTypeCategory), u32> = BTreeMap::new();

    for (_, operator_rewardables) in node_operators_rewardables {
        for (node_type, node_count) in operator_rewardables.rewardables {
            node_providers_rewardables
                .entry((operator_rewardables.node_provider_id, (operator_rewardables.region.clone(), node_type)))
                .and_modify(|count| *count += node_count)
                .or_insert(node_count);
        }
    }

    node_providers_rewardables
}

pub async fn get_rewards_table() -> anyhow::Result<NodeRewardsTable> {
    ic_cdk::println!("Fetching NodeRewardsTable from registry canister");
    let (rewards_table, _): (NodeRewardsTable, _) = ic_nns_common::registry::get_value(NODE_REWARDS_TABLE_KEY.as_bytes(), None).await?;
    Ok(rewards_table)
}
