use crate::metrics::MetricsManager;
use crate::metrics_types::{StorableSubnetMetricsKey, SubnetNodeMetricsDaily};
use futures::StreamExt;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_cdk::api::call::{CallResult, RejectionCode};
use ic_management_canister_types_private::{NodeMetrics, NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::memory_manager::{MemoryId, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;

mod mock {
    use super::*;
    use crate::metrics::ManagementCanisterClient;
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        #[derive(Debug)]
        pub CanisterClient {}

        #[async_trait]
        impl ManagementCanisterClient for CanisterClient {
            async fn node_metrics_history(&self, contract: ic_management_canister_types_private::NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
        }
    }
}

pub type VM = VirtualMemory<DefaultMemoryImpl>;
const ONE_DAY_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
fn subnet_id(id: u64) -> ic_base_types::SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}
fn node_id(id: u64) -> ic_base_types::NodeId {
    PrincipalId::new_node_test_id(id).into()
}

impl MetricsManager<VM> {
    fn new(client: mock::MockCanisterClient) -> Self {
        Self {
            client: Box::new(client),
            subnets_metrics: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(0))),
            subnets_to_retry: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(1))),
            last_timestamp_per_subnet: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(2))),
        }
    }
}

fn node_metrics_history_gen(days: u64) -> Vec<NodeMetricsHistoryResponse> {
    let mut result = Vec::new();
    for i in 0..days {
        result.push(NodeMetricsHistoryResponse {
            timestamp_nanos: i * ONE_DAY_NANOS,
            ..Default::default()
        });
    }
    result
}

#[tokio::test]
async fn subnet_metrics_added_correctly() {
    let days = 45;
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days),)));
    let mm = MetricsManager::new(mock);

    let subnet_1 = subnet_id(1);

    mm.update_subnets_metrics(vec![subnet_1]).await;
    for i in 0..days {
        let key = StorableSubnetMetricsKey {
            timestamp_nanos: i * ONE_DAY_NANOS,
            subnet_id: subnet_1,
        };
        assert!(mm.subnets_metrics.borrow().get(&key).is_some());
    }
}

#[tokio::test]
async fn subnets_to_retry_filled() {
    let subnet_1 = subnet_id(1);
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .times(1)
        .return_const(CallResult::Err((RejectionCode::Unknown, "Error".to_string())));
    mock.expect_node_metrics_history()
        .times(1)
        .return_const(CallResult::Ok((node_metrics_history_gen(2),)));

    let mm = MetricsManager::new(mock);
    mm.update_subnets_metrics(vec![subnet_1]).await;
    assert_eq!(mm.subnets_to_retry.borrow().get(&subnet_1.into()), Some(1));

    // Retry the subnet and success
    mm.update_subnets_metrics(vec![subnet_1]).await;
    assert_eq!(mm.subnets_to_retry.borrow().get(&subnet_1.into()), None);
}

#[tokio::test]
async fn multiple_subnets_metrics_added_correctly() {
    let days = 30;
    let mut mock = mock::MockCanisterClient::new();

    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days),)));
    let mm = MetricsManager::new(mock);
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);

    mm.update_subnets_metrics(vec![subnet_1, subnet_2]).await;

    for subnet in &[subnet_1, subnet_2] {
        for i in 0..days {
            let key = StorableSubnetMetricsKey {
                timestamp_nanos: i * ONE_DAY_NANOS,
                subnet_id: *subnet,
            };
            assert!(mm.subnets_metrics.borrow().get(&key).is_some(), "Metrics missing for subnet {:?}", subnet);
        }
    }
}

#[tokio::test]
async fn retry_count_increments_on_failure() {
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .return_const(CallResult::Err((RejectionCode::Unknown, "Temporary error".to_string())));

    let mm = MetricsManager::new(mock);
    let subnet_1 = subnet_id(1);

    for retry_attempt in 1..=3 {
        mm.update_subnets_metrics(vec![subnet_1]).await;
        assert_eq!(
            mm.subnets_to_retry.borrow().get(&subnet_1.into()),
            Some(retry_attempt),
            "Retry count should be {}",
            retry_attempt
        );
    }
}

#[tokio::test]
async fn no_metrics_added_when_call_fails() {
    let mut mock = mock::MockCanisterClient::new();
    let subnet_1 = subnet_id(1);

    mock.expect_node_metrics_history()
        .return_const(CallResult::Err((RejectionCode::Unknown, "Error".to_string())));
    let mm = MetricsManager::new(mock);

    mm.update_subnets_metrics(vec![subnet_1]).await;

    assert!(mm.subnets_metrics.borrow().is_empty(), "Metrics should be empty after a failed call");
}

#[tokio::test]
async fn partial_failures_are_handled_correctly() {
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history().returning(move |subnet| {
        if SubnetId::from(subnet.subnet_id) == subnet_1 {
            CallResult::Err((RejectionCode::Unknown, "Error".to_string()))
        } else {
            CallResult::Ok((node_metrics_history_gen(1),))
        }
    });

    let mm = MetricsManager::new(mock);

    mm.update_subnets_metrics(vec![subnet_1, subnet_2]).await;

    assert_eq!(
        mm.subnets_to_retry.borrow().get(&subnet_1.into()),
        Some(1),
        "Subnet 1 should be in retry list"
    );
    assert!(
        mm.subnets_to_retry.borrow().get(&subnet_2.into()).is_none(),
        "Subnet 2 should not be in retry list"
    );

    let key = StorableSubnetMetricsKey {
        timestamp_nanos: 0,
        subnet_id: subnet_1,
    };
    assert!(
        mm.subnets_metrics.borrow().get(&key).is_none(),
        "Metrics should not be present for subnet 1"
    );

    let key = StorableSubnetMetricsKey {
        timestamp_nanos: 0,
        subnet_id: subnet_2,
    };
    assert!(mm.subnets_metrics.borrow().get(&key).is_some(), "Metrics should be present for subnet 2");
}

fn node_metrics_history(node_id: NodeId, proposed_failed_blocks: Vec<(u64, u64)>) -> Vec<NodeMetricsHistoryResponse> {
    let mut result = Vec::new();
    for (idx, (num_proposed, num_failed)) in proposed_failed_blocks.into_iter().enumerate() {
        result.push(NodeMetricsHistoryResponse {
            timestamp_nanos: idx as u64 * ONE_DAY_NANOS,
            node_metrics: vec![NodeMetrics {
                num_blocks_proposed_total: num_proposed,
                num_block_failures_total: num_failed,
                node_id: node_id.get(),
            }],
        });
    }
    result
}

const MAX_TIMES: usize = 20;

#[derive(Clone)]
struct NodeMetricsHistoryResponseTracker {
    current_subnet: SubnetId,
    current_node: NodeId,
    current_ts: u64,
    subnets_responses: HashMap<SubnetId, Vec<NodeMetricsHistoryResponse>>,
}

impl NodeMetricsHistoryResponseTracker {
    pub fn new() -> Self {
        Self {
            current_subnet: subnet_id(0),
            current_node: node_id(0),
            current_ts: 0,
            subnets_responses: HashMap::new(),
        }
    }

    fn with_subnet(mut self, subnet_id: SubnetId) -> Self {
        self.subnets_responses.entry(subnet_id).or_default();
        self.current_subnet = subnet_id;
        self
    }

    fn with_node(mut self, node_id: NodeId) -> Self {
        self.current_node = node_id;
        self
    }

    fn change_ts(mut self, ts: u64) -> Self {
        self.current_ts = ts;
        self.subnets_responses
            .get_mut(&self.current_subnet)
            .unwrap()
            .push(NodeMetricsHistoryResponse {
                timestamp_nanos: ts,
                node_metrics: Vec::new(),
            });
        self
    }

    fn with_node_metrics_single(mut self, blocks_proposed_total: u64, blocks_failed_total: u64) -> Self {
        let responses = self.subnets_responses.get_mut(&self.current_subnet).unwrap();

        // Ensure we have at least one response with the current timestamp
        if responses.is_empty() || responses.last().unwrap().timestamp_nanos != self.current_ts {
            responses.push(NodeMetricsHistoryResponse {
                timestamp_nanos: self.current_ts,
                node_metrics: Vec::new(),
            });
        }

        // Now we can safely add the metrics to the last response
        responses.last_mut().unwrap().node_metrics.push(NodeMetrics {
            num_blocks_proposed_total: blocks_proposed_total,
            num_block_failures_total: blocks_failed_total,
            node_id: self.current_node.get(),
        });

        self
    }
    fn add_node_metrics(mut self, proposed_failed: Vec<(u64, u64)>) -> Self {
        let len = proposed_failed.len();
        for (i, (proposed, failed)) in proposed_failed.into_iter().enumerate() {
            self = self.with_node_metrics_single(proposed, failed);
            if i < len - 1 {
                let new_ts = self.current_ts + ONE_DAY_NANOS;
                self = self.change_ts(new_ts);
            }
        }
        self
    }

    fn next(&self, response_step: usize, contract: NodeMetricsHistoryArgs) -> Vec<NodeMetricsHistoryResponse> {
        self.subnets_responses
            .get(&SubnetId::from(contract.subnet_id))
            .unwrap()
            .clone()
            .into_iter()
            .filter(|metric| metric.timestamp_nanos >= contract.start_at_timestamp_nanos)
            .take(response_step)
            .collect()
    }
}

async fn daily_metrics_are_correctly_computed(response_step: usize) {
    // (blocks_proposed, blocks_failed)
    let tracker = NodeMetricsHistoryResponseTracker::new()
        .with_subnet(subnet_id(1))
        .with_node(node_id(1))
        .add_node_metrics(vec![(7, 5), (10, 6), (15, 6), (25, 50), (10, 6)]);

    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history().times(MAX_TIMES).returning(move |contract| {
        let call_elem = tracker.next(response_step, contract);

        println!("{:?}", call_elem);
        CallResult::Ok((call_elem,))
    });
    let mm = MetricsManager::new(mock);

    for _ in 0..MAX_TIMES {
        mm.update_subnets_metrics(vec![subnet_id(1)]).await;
    }
    let node_1_daily_metrics: Vec<SubnetNodeMetricsDaily> = mm
        .subnets_metrics
        .borrow()
        .values()
        .map(|node_metrics| node_metrics.0[0].clone())
        .collect();

    // (7, 5)
    assert_eq!(node_1_daily_metrics[0].num_blocks_proposed, 7);
    assert_eq!(node_1_daily_metrics[0].num_blocks_failed, 5);

    // (10 - 7, 6 - 5) = (5, 1)
    assert_eq!(node_1_daily_metrics[1].num_blocks_proposed, 3);
    assert_eq!(node_1_daily_metrics[1].num_blocks_failed, 1);

    // (15 - 10, 6 - 6) = (5, 0)
    assert_eq!(node_1_daily_metrics[2].num_blocks_proposed, 5);
    assert_eq!(node_1_daily_metrics[2].num_blocks_failed, 0);

    // (25 - 15, 50 - 6) = (10, 44)
    assert_eq!(node_1_daily_metrics[3].num_blocks_proposed, 10);
    assert_eq!(node_1_daily_metrics[3].num_blocks_failed, 44);

    // Node is redeployed and added to the same subnet!
    // (10, 6)
    assert_eq!(node_1_daily_metrics[4].num_blocks_proposed, 10);
    assert_eq!(node_1_daily_metrics[4].num_blocks_failed, 6);
}

#[tokio::test]
async fn daily_metrics_are_correctly_computed_over_multiple_updates() {
    daily_metrics_are_correctly_computed(2).await;
    daily_metrics_are_correctly_computed(3).await;
    daily_metrics_are_correctly_computed(4).await;
    daily_metrics_are_correctly_computed(5).await;
}
