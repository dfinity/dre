use std::collections::{btree_map::Entry, BTreeMap};

use anyhow::Ok;
use dfn_core::api::PrincipalId;
use futures::FutureExt;
use ic_management_canister_types::NodeMetrics as ICManagementNodeMetrics;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use itertools::Itertools;

use crate::{
    stable_memory,
    types::{NodeMetrics, PrincipalNodeMetricsHistory, SubnetNodeMetrics, TimestampNanos},
};

impl SubnetNodeMetrics {
    pub fn new(subnet_id: PrincipalId, subnet_metrics: Vec<ICManagementNodeMetrics>) -> Self {
        let node_metrics = subnet_metrics.into_iter().map(|node_metrics| node_metrics.into()).collect_vec();

        Self {
            subnet_id: subnet_id.0,
            node_metrics,
        }
    }
}

impl From<ICManagementNodeMetrics> for NodeMetrics {
    fn from(node_metrics: ICManagementNodeMetrics) -> Self {
        Self {
            node_id: node_metrics.node_id.0,
            num_block_failures_total: node_metrics.num_block_failures_total,
            num_blocks_proposed_total: node_metrics.num_blocks_proposed_total,
        }
    }
}

fn store_results(results: BTreeMap<u64, Vec<SubnetNodeMetrics>>) {
    for (timestamp, storable) in results {
        stable_memory::insert(timestamp, storable)
    }
}

fn transform_metrics(subnets_metrics: Vec<PrincipalNodeMetricsHistory>) -> BTreeMap<TimestampNanos, Vec<SubnetNodeMetrics>> {
    let mut results = BTreeMap::new();

    for (subnet, subnet_metrics) in subnets_metrics {
        for ts_node_metrics in subnet_metrics {
            let ts: TimestampNanos = ts_node_metrics.timestamp_nanos;

            let subnet_metrics_storable = SubnetNodeMetrics::new(subnet, ts_node_metrics.node_metrics);

            match results.entry(ts) {
                Entry::Occupied(mut entry) => {
                    let v: &mut Vec<SubnetNodeMetrics> = entry.get_mut();
                    v.push(subnet_metrics_storable)
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![subnet_metrics_storable]);
                }
            }
        }
    }
    results
}

async fn fetch_metrics(subnets: Vec<PrincipalId>, refresh_ts: TimestampNanos) -> anyhow::Result<Vec<PrincipalNodeMetricsHistory>> {
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
                .map_err(|(code, msg)| anyhow::anyhow!("Error when calling management canister:\n Code:{:?}\nMsg:{}", code, msg))
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

async fn fetch_subnets() -> anyhow::Result<Vec<PrincipalId>> {
    let (registry_subnets, _): (SubnetListRecord, _) = ic_nns_common::registry::get_value("subnet_list".as_bytes(), None).await?;
    let subnets = registry_subnets
        .subnets
        .into_iter()
        .map(|subnet_id: Vec<u8>| PrincipalId::try_from(subnet_id).unwrap())
        .collect_vec();

    Ok(subnets)
}

pub async fn update_metrics() -> anyhow::Result<()> {
    let subnets = fetch_subnets().await?;
    let latest_ts = stable_memory::latest_key().unwrap_or_default();
    let refresh_ts = latest_ts + 1;

    ic_cdk::println!(
        "Updating metrics for subnets: {:?}\nLatest timestamp persisted: {}\nRefreshing metrics from timestamp {}",
        subnets,
        latest_ts,
        refresh_ts
    );

    let metrics = fetch_metrics(subnets, refresh_ts).await?;

    let results = transform_metrics(metrics);

    store_results(results);

    ic_cdk::println!("Successfully stored metrics");

    Ok(())
}
