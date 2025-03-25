use crate::metrics_types::{StorableNodeMetrics, StorableNodeMetricsKey, SubnetIdStored, MAX_PRINCIPAL_ID};
use async_trait::async_trait;
use ic_base_types::{NodeId, SubnetId};
use ic_cdk::api::call::CallResult;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use itertools::Itertools;
use node_provider_rewards::metrics::NodeDailyMetrics;

pub type RetryCount = u64;
pub type TimestampNanos = u64;

#[async_trait]
pub trait ManagementCanisterClient {
    async fn node_metrics_history(&self, args: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
}

pub struct MetricsManager<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub(crate) client: Box<dyn ManagementCanisterClient>,
    pub(crate) nodes_metrics: RefCell<StableBTreeMap<StorableNodeMetricsKey, StorableNodeMetrics, Memory>>,
    pub(crate) subnets_to_retry: RefCell<StableBTreeMap<SubnetIdStored, RetryCount, Memory>>,
    pub(crate) last_timestamp_per_subnet: RefCell<StableBTreeMap<SubnetIdStored, TimestampNanos, Memory>>,
}

impl<Memory> MetricsManager<Memory>
where
    Memory: ic_stable_structures::Memory + 'static,
{
    pub async fn retry_failed_subnets(&self) {
        let subnets_to_retry: Vec<SubnetId> = self.subnets_to_retry.borrow().keys().map(|key| *key).collect();

        if !subnets_to_retry.is_empty() {
            ic_cdk::println!("Retrying metrics for subnets: {:?}", subnets_to_retry);
            self.update_subnets_metrics(subnets_to_retry).await;
        }
    }

    /// Fetches subnets metrics for the specified subnets from their last timestamp.
    async fn fetch_subnets_metrics(
        &self,
        last_timestamp_per_subnet: &BTreeMap<SubnetId, TimestampNanos>,
    ) -> BTreeMap<SubnetId, CallResult<(Vec<NodeMetricsHistoryResponse>,)>> {
        let mut subnets_node_metrics = Vec::new();

        for (subnet_id, last_metrics_ts) in last_timestamp_per_subnet {
            let refresh_ts = last_metrics_ts + 1;
            ic_cdk::println!(
                "Updating node metrics for subnet {}: Latest timestamp persisted: {}  Refreshing metrics from timestamp {}",
                subnet_id,
                last_metrics_ts,
                refresh_ts
            );

            let contract = NodeMetricsHistoryArgs {
                subnet_id: subnet_id.get(),
                start_at_timestamp_nanos: refresh_ts,
            };

            subnets_node_metrics.push(async move { (*subnet_id, self.client.node_metrics_history(contract).await) });
        }

        futures::future::join_all(subnets_node_metrics).await.into_iter().collect()
    }

    /// Updates the stored subnets metrics from remote management canisters.
    ///
    /// This function fetches the nodes metrics for the given subnets from the management canisters
    /// updating the local metrics with the fetched metrics.
    pub async fn update_subnets_metrics(&self, subnets: Vec<SubnetId>) {
        let last_timestamp_per_subnet: BTreeMap<SubnetId, TimestampNanos> = subnets
            .into_iter()
            .map(|subnet| {
                let last_timestamp = self.last_timestamp_per_subnet.borrow().get(&subnet.into());

                (subnet, last_timestamp.unwrap_or_default())
            })
            .collect();

        let subnets_metrics = self.fetch_subnets_metrics(&last_timestamp_per_subnet).await;
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok((metrics_history,)) => {
                    // Update the last timestamp for this subnet.
                    let last_timestamp = metrics_history
                        .iter()
                        .map(|entry| entry.timestamp_nanos)
                        .max()
                        .unwrap_or(*last_timestamp_per_subnet.get(&subnet_id).expect("timestamp exists"));

                    self.last_timestamp_per_subnet.borrow_mut().insert(subnet_id.into(), last_timestamp);

                    // Insert each fetched metric entry into our node metrics map.
                    metrics_history.into_iter().for_each(|metrics| {
                        let mut stable_node_metrics = self.nodes_metrics.borrow_mut();

                        for node_metrics in metrics.node_metrics {
                            let key = StorableNodeMetricsKey {
                                ts: metrics.timestamp_nanos,
                                node_id: node_metrics.node_id.into(),
                                subnet_assigned: subnet_id,
                            };

                            stable_node_metrics.insert(key, StorableNodeMetrics(node_metrics));
                        }
                    });

                    // Remove the subnet from the retry list if present.
                    self.subnets_to_retry.borrow_mut().remove(&subnet_id.into());
                }
                Err((code, msg)) => {
                    ic_cdk::println!("Error fetching metrics for subnet {}: CODE: {:?} MSG: {}", subnet_id, code, msg);

                    // The call failed, will retry fetching metrics for this subnet.
                    let mut retry_count = self.subnets_to_retry.borrow().get(&subnet_id.into()).unwrap_or_default();
                    retry_count += 1;

                    self.subnets_to_retry.borrow_mut().insert(subnet_id.into(), retry_count);
                }
            }
        }
    }

    /// Fetches subnets metrics for the specified subnets from their last timestamp.
    pub fn get_daily_metrics_by_node(&self, start_ts: TimestampNanos, end_ts: TimestampNanos) -> BTreeMap<NodeId, Vec<NodeDailyMetrics>> {
        let first_key = StorableNodeMetricsKey {
            ts: start_ts,
            ..Default::default()
        };
        let last_key = StorableNodeMetricsKey {
            ts: end_ts,
            node_id: NodeId::from(MAX_PRINCIPAL_ID.clone()),
            subnet_assigned: SubnetId::from(MAX_PRINCIPAL_ID.clone())
        };

        // Group metrics by node_id in the given time range
        let nodes_in_range: HashMap<NodeId, _> = self
            .nodes_metrics
            .borrow()
            .range(first_key.clone()..=last_key)
            .into_iter()
            .into_group_map_by(|(key, _)| key.node_id);

        // Find the first metrics before start_ts for each node
        let mut first_metrics: HashMap<NodeId, Option<NodeDailyMetrics>> = HashMap::new();

        for node_id in nodes_in_range.keys() {
            let first = self
                .nodes_metrics
                .borrow()
                .range(..first_key.clone())
                .filter(|(key, _)| key.node_id == *node_id)
                .rev()
                .next()
                .map(|(_, metrics)| metrics.clone());

            first_metrics.insert(*node_id, first);
        }

        // Return the daily metrics
        first_metrics.into_iter().collect()
        
            .flat_map(|(key, node_metrics)| {
                (key.node_id, (key, node_metrics.))
            })
            // TODO: Check if order preserved
            .into_group_map_by(|(_, _, metrics)| metrics.node_id)
            .into_iter()
            .map(|(node_id, entries)| {
                let (daily_metrics, _) = entries.into_iter().fold(
                    (Vec::new(), (0, 0)),
                    |(mut acc, (prev_proposed, prev_failed)), (ts, subnet_id, node_metrics)| {
                        let current_proposed = node_metrics.num_blocks_proposed_total;
                        let current_failed = node_metrics.num_block_failures_total;

                        let (num_blocks_proposed, num_blocks_failed) = if prev_proposed > current_proposed || prev_failed > current_failed {
                            (current_proposed, current_failed)
                        } else {
                            (current_proposed - prev_proposed, current_failed - prev_failed)
                        };

                        acc.push(NodeDailyMetrics::new(ts, subnet_id, num_blocks_proposed, num_blocks_failed));
                        (acc, (current_proposed, current_failed))
                    },
                );

                (NodeId::from(node_id), daily_metrics)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
