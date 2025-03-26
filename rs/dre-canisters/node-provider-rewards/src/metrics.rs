use crate::metrics_types::{StorableSubnetMetrics, StorableSubnetMetricsKey, SubnetIdStored};
use async_trait::async_trait;
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types_private::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use std::cell::RefCell;
use std::collections::BTreeMap;

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
    pub(crate) subnets_metrics: RefCell<StableBTreeMap<StorableSubnetMetricsKey, StorableSubnetMetrics, Memory>>,
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
                let last_timestamp = self.last_timestamp_per_subnet.borrow().get(&SubnetIdStored(subnet));

                (subnet, last_timestamp.unwrap_or_default())
            })
            .collect();

        let subnets_metrics = self.fetch_subnets_metrics(&last_timestamp_per_subnet).await;
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok((nodes_metrics_history,)) => {
                    // Update the last timestamp for this subnet.
                    let last_timestamp = nodes_metrics_history
                        .iter()
                        .map(|entry| entry.timestamp_nanos)
                        .max()
                        .unwrap_or(*last_timestamp_per_subnet.get(&subnet_id).expect("timestamp exists"));

                    self.last_timestamp_per_subnet.borrow_mut().insert(subnet_id.into(), last_timestamp);

                    // Insert each fetched metric entry into our node metrics map.
                    nodes_metrics_history.into_iter().for_each(|entry| {
                        let key = StorableSubnetMetricsKey {
                            subnet_id,
                            timestamp_nanos: entry.timestamp_nanos,
                        };
                        self.subnets_metrics.borrow_mut().insert(key, StorableSubnetMetrics(entry.node_metrics));
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
}

#[cfg(test)]
mod tests;
