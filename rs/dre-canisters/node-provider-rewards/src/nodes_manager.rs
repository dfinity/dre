use std::collections::{BTreeMap, HashSet};

use candid::Principal;
use futures::FutureExt;
use ic_base_types::NodeId;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use ic_registry_keys::{
    make_data_center_record_key, make_node_operator_record_key, make_node_record_key, make_subnet_list_record_key, NODE_REWARDS_TABLE_KEY,
};
use ic_registry_transport::{deserialize_get_value_response, serialize_get_value_request};
use ic_types::PrincipalId;

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

async fn get_most_recent_registry_values<T: prost::Message + Default>(keys: Vec<String>) -> anyhow::Result<BTreeMap<String, T>> {
    let latest_registry_version = ic_nns_common::registry::get_latest_version().await;
    let mut buffer = Vec::new();
    let mut keys_to_retry = Vec::new();
    let mut registry_values = BTreeMap::new();

    for (index, key) in keys.iter().enumerate() {
        let get_value_request = serialize_get_value_request(key.as_bytes().to_vec(), Some(latest_registry_version)).unwrap();
        let fut_value = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_value", get_value_request, 0)
        .map(|res| res.map(|result| (key.clone(), result)));

        buffer.push(fut_value);

        if buffer.len() > 100 || index == keys.len() - 1 {
            let buffer_batch = std::mem::take(&mut buffer);
            let batch_results = futures::future::try_join_all(buffer_batch)
                .await
                .map_err(|(code, msg)| anyhow::anyhow!("Error when calling registry canister:\n Code:{:?}\nMsg:{}", code, msg))?;

            for (key, result_ok) in batch_results {
                let deserialized_registry_value =
                    deserialize_get_value_response(result_ok).map(|(response, _)| T::decode(response.as_slice()).unwrap());
                match deserialized_registry_value {
                    Ok(registry_value) => {
                        ic_cdk::println!("Fetched key from registry: {}", key);
                        registry_values.insert(key, registry_value);
                    }
                    Err(_) => keys_to_retry.push(key),
                }
            }
        }
    }

    for key in keys_to_retry {
        for registry_version in (0..=latest_registry_version).rev() {
            ic_cdk::println!("Trying registry version: {}", registry_version);

            match ic_nns_common::registry::get_value::<T>(key.as_bytes(), Some(registry_version)).await {
                Ok((registry_value, _)) => {
                    ic_cdk::println!("Fetched key from registry: {}", key);
                    registry_values.insert(key, registry_value);
                    break;
                }
                Err(e) => {
                    ic_cdk::println!("{}", e);
                }
            }
        }
    }

    anyhow::Ok(registry_values)
}

pub async fn get_node_operators_rewardables(rewarding_period: &DateTimeRange) -> anyhow::Result<BTreeMap<PrincipalId, RewardableNodes>> {
    // let (registry_node_operators, _): (NodeOperatorListRecord, _) =
    //     ic_nns_common::registry::get_value("node_operator_list".as_bytes(), None).await?;
    // let node_operators = registry_node_operators
    //     .node_operators
    //     .into_iter()
    //     .map(|node_operator_id: Vec<u8>| PrincipalId::try_from(node_operator_id))
    //     .collect::<Result<Vec<_>, _>>()?;

    let mut rewardables = BTreeMap::new();

    let node_id_keys = get_daily_metrics(rewarding_period)
        .into_keys()
        .map(|principal| make_node_record_key(NodeId::from(PrincipalId::from(principal))))
        .collect_vec();

    let node_operators_keys = get_most_recent_registry_values::<NodeRecord>(node_id_keys)
        .await?
        .into_iter()
        .flat_map(|(_, node_record)| node_record.node_operator_id.try_into())
        .map(make_node_operator_record_key)
        .collect_vec();

    let node_operator_records = get_most_recent_registry_values::<NodeOperatorRecord>(node_operators_keys).await?;

    let dcs_keys: HashSet<String> = node_operator_records
        .values()
        .map(|node_operator_record| make_data_center_record_key(node_operator_record.dc_id.as_str()))
        .collect();
    let dcs_records = get_most_recent_registry_values::<DataCenterRecord>(dcs_keys.into_iter().collect()).await?;

    for (_, node_operator_record) in node_operator_records {
        let dc_key = make_data_center_record_key(node_operator_record.dc_id.as_str());
        let region = dcs_records.get(&dc_key).unwrap().region.clone();

        let node_provider_id: PrincipalId = match node_operator_record.node_provider_principal_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                ic_cdk::println!("Error converting node provider ID {:?}", e);
                continue;
            }
        };

        let node_operator_id: PrincipalId = match node_operator_record.node_operator_principal_id.try_into() {
            Ok(id) => id,
            Err(e) => {
                ic_cdk::println!("Error converting node operator ID {:?}", e);
                continue;
            }
        };

        rewardables.insert(
            node_operator_id,
            RewardableNodes {
                node_provider_id: node_provider_id.0,
                region,
                rewardables: node_operator_record.rewardable_nodes,
            },
        );
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
    let daily_metrics: BTreeMap<Principal, Vec<DailyNodeMetrics>> = get_daily_metrics(rewarding_period);

    let node_id_keys = get_daily_metrics(rewarding_period)
        .keys()
        .cloned()
        .map(|principal| make_node_record_key(NodeId::from(PrincipalId::from(principal))))
        .collect_vec();

    let node_operators_ids: BTreeMap<String, PrincipalId> = get_most_recent_registry_values::<NodeRecord>(node_id_keys)
        .await?
        .into_iter()
        .map(|(node_id_key, node_record)| (node_id_key, node_record.node_operator_id.try_into().unwrap()))
        .collect();

    for (node_principal, daily_metrics) in daily_metrics {
        let node_id = PrincipalId::from(node_principal);

        let node_operator_id = node_operators_ids.get(&make_node_record_key(NodeId::from(node_id))).unwrap();
        let operator_rewardables = rewardable_nodes.get_mut(node_operator_id).unwrap();

        let node_type = if operator_rewardables.rewardables.is_empty() {
            "unknown:no_rewardable_nodes_found".to_string()
        } else {
            // Find the first non-zero rewardable node type, or "unknown" if none are found
            let (k, mut v) = loop {
                let (k, v) = match operator_rewardables.rewardables.pop_first() {
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
                operator_rewardables.rewardables.insert(k.clone(), v);
            }
            k
        };

        nodes_metadata.insert(
            node_principal,
            (
                NodeMetadata {
                    node_provider_id: operator_rewardables.node_provider_id,
                    region: operator_rewardables.region.clone(),
                    node_type,
                },
                daily_metrics,
            ),
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
