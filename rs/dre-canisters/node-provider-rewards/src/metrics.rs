use crate::metrics_types::{StorableSubnetMetrics, StorableSubnetMetricsKey, SubnetIdStored};
use async_trait::async_trait;
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub type RetryCount = u64;
pub type TimestampNanos = u64;
pub type StableSubnetsToRetry<Memory> = StableBTreeMap<SubnetIdStored, RetryCount, Memory>;
pub type StableSubnetsMetrics<Memory> = StableBTreeMap<StorableSubnetMetricsKey, StorableSubnetMetrics, Memory>;
pub type StableLastTimestampPerSubnet<Memory> = StableBTreeMap<SubnetIdStored, TimestampNanos, Memory>;

pub trait MetricsManagerData<Memory: ic_stable_structures::Memory> {
    fn with_subnets_to_retry<R>(f: impl FnOnce(&StableSubnetsToRetry<Memory>) -> R) -> R;
    fn with_subnets_to_retry_mut<R>(f: impl FnOnce(&mut StableSubnetsToRetry<Memory>) -> R) -> R;

    fn with_subnets_metrics<R>(f: impl FnOnce(&StableSubnetsMetrics<Memory>) -> R) -> R;
    fn with_subnets_metrics_mut<R>(f: impl FnOnce(&mut StableSubnetsMetrics<Memory>) -> R) -> R;

    fn with_last_timestamp_per_subnet<R>(f: impl FnOnce(&StableLastTimestampPerSubnet<Memory>) -> R) -> R;
    fn with_last_timestamp_per_subnet_mut<R>(f: impl FnOnce(&mut StableLastTimestampPerSubnet<Memory>) -> R) -> R;
}

#[async_trait]
pub trait ManagementCanisterCaller {
    async fn node_metrics_history(&self, args: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
}

pub struct MetricsManager<D: MetricsManagerData<Memory>, Memory>
where
    Memory: ic_stable_structures::Memory,
{
    _metrics_manager_data: PhantomData<D>,
    _memory: PhantomData<Memory>,
}

impl<D: MetricsManagerData<Memory>, Memory> MetricsManager<D, Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub async fn retry_failed_subnets<M: ManagementCanisterCaller>(runtime: &M) {
        let subnets_to_retry: Vec<SubnetId> = D::with_subnets_to_retry(|subnets_to_retry| subnets_to_retry.keys().map(|key| *key).collect());

        if !subnets_to_retry.is_empty() {
            ic_cdk::println!("Retrying metrics for subnets: {:?}", subnets_to_retry);
            Self::update_subnets_metrics(runtime, subnets_to_retry).await
        }
    }

    /// Fetches subnets metrics for the specified subnets from their last timestamp.
    async fn fetch_subnets_metrics<M: ManagementCanisterCaller>(
        runtime: &M,
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

            subnets_node_metrics.push(async move {
                let call_result = runtime.node_metrics_history(contract).await;

                (*subnet_id, call_result)
            });
        }

        futures::future::join_all(subnets_node_metrics).await.into_iter().collect()
    }

    /// Updates the stored subnets metrics from remote management canisters.
    ///
    /// This function fetches the nodes metrics for the given subnets from the management canisters
    /// updating the local metrics with the fetched metrics.
    pub async fn update_subnets_metrics<M: ManagementCanisterCaller>(runtime: &M, subnets: Vec<SubnetId>) {
        let last_timestamp_per_subnet = D::with_last_timestamp_per_subnet(|last_timestamp_per_subnet| {
            subnets
                .into_iter()
                .map(|subnet| {
                    let last_timestamp = last_timestamp_per_subnet.get(&SubnetIdStored(subnet));

                    (subnet, last_timestamp.unwrap_or_default())
                })
                .collect()
        });

        let subnets_metrics = Self::fetch_subnets_metrics(runtime, &last_timestamp_per_subnet).await;
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok((nodes_metrics_history,)) => {
                    // Update the last timestamp for this subnet.
                    let last_timestamp = nodes_metrics_history
                        .iter()
                        .map(|entry| entry.timestamp_nanos)
                        .max()
                        .unwrap_or(*last_timestamp_per_subnet.get(&subnet_id).expect("timestamp exists"));

                    D::with_last_timestamp_per_subnet_mut(|last_timestamp_per_subnet| {
                        last_timestamp_per_subnet.insert(subnet_id.into(), last_timestamp)
                    });

                    // Insert each fetched metric entry into our node metrics map.
                    D::with_subnets_metrics_mut(|subnets_metrics| {
                        nodes_metrics_history.into_iter().for_each(|entry| {
                            let key = StorableSubnetMetricsKey {
                                subnet_id,
                                timestamp_nanos: entry.timestamp_nanos,
                            };
                            subnets_metrics.insert(key, StorableSubnetMetrics(entry.node_metrics));
                        });
                    });

                    // Remove the subnet from the retry list if present.
                    D::with_subnets_to_retry_mut(|subnets_to_retry| subnets_to_retry.remove(&subnet_id.into()));
                }
                Err((code, msg)) => {
                    ic_cdk::println!("Error fetching metrics for subnet {}: CODE: {:?} MSG: {}", subnet_id, code, msg);

                    // The call failed, will retry fetching metrics for this subnet.
                    D::with_subnets_to_retry_mut(|subnets_to_retry| {
                        let mut retry_count = subnets_to_retry.get(&subnet_id.into()).unwrap_or_default();
                        retry_count += 1;

                        subnets_to_retry.insert(subnet_id.into(), retry_count)
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
