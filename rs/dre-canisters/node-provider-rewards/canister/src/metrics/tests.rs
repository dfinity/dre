use crate::metrics::{MetricsManager, TimestampNanos};
use crate::metrics_types::{NodeMetricsDailyStored, SubnetMetricsStoredKey};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_cdk::api::call::{CallResult, RejectionCode};
use ic_management_canister_types_private::{NodeMetrics, NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::memory_manager::{MemoryId, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use rewards_calculation::reward_period::DayEndNanos;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

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
            async fn node_metrics_history(&self, contract: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
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
        let key = SubnetMetricsStoredKey {
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
            let key = SubnetMetricsStoredKey {
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

    let key = SubnetMetricsStoredKey {
        timestamp_nanos: 0,
        subnet_id: subnet_1,
    };
    assert!(
        mm.subnets_metrics.borrow().get(&key).is_none(),
        "Metrics should not be present for subnet 1"
    );

    let key = SubnetMetricsStoredKey {
        timestamp_nanos: 0,
        subnet_id: subnet_2,
    };
    assert!(mm.subnets_metrics.borrow().get(&key).is_some(), "Metrics should be present for subnet 2");
}

const MAX_TIMES: usize = 20;
type FromTS = u64;
type Proposed = u64;
type Failed = u64;

#[derive(Clone)]
struct NodeMetricsHistoryResponseTracker {
    current_subnet: SubnetId,
    subnets_responses: BTreeMap<TimestampNanos, HashMap<SubnetId, Vec<NodeMetrics>>>,
}

impl NodeMetricsHistoryResponseTracker {
    pub fn new() -> Self {
        Self {
            current_subnet: subnet_id(0),
            subnets_responses: BTreeMap::new(),
        }
    }

    fn with_subnet(mut self, subnet_id: SubnetId) -> Self {
        self.current_subnet = subnet_id;
        for (_, metrics) in self.subnets_responses.iter_mut() {
            metrics.insert(subnet_id, Vec::new());
        }
        self
    }

    fn add_node_metrics(mut self, node_id: NodeId, metrics: Vec<(FromTS, Vec<(Proposed, Failed)>)>) -> Self {
        for (from_ts, proposed_failed) in metrics {
            for (i, (proposed, failed)) in proposed_failed.into_iter().enumerate() {
                let ts = from_ts + (i as u64) * ONE_DAY_NANOS;
                let entry = self.subnets_responses.entry(ts).or_default();
                let entry_sub = entry.entry(self.current_subnet).or_default();

                entry_sub.push(NodeMetrics {
                    num_blocks_proposed_total: proposed,
                    num_block_failures_total: failed,
                    node_id: node_id.get(),
                });
            }
        }
        self
    }

    fn next(&self, response_step: usize, contract: NodeMetricsHistoryArgs) -> Vec<NodeMetricsHistoryResponse> {
        let mut response = Vec::new();
        self.subnets_responses
            .range(contract.start_at_timestamp_nanos..(contract.start_at_timestamp_nanos + (response_step as u64) * ONE_DAY_NANOS))
            .filter(|(_, metrics)| metrics.contains_key(&contract.subnet_id.into()))
            .for_each(|(ts, metrics)| {
                let node_metrics = metrics.get(&contract.subnet_id.into()).unwrap().clone();
                response.push(NodeMetricsHistoryResponse {
                    node_metrics,
                    timestamp_nanos: *ts,
                });
            });

        response
    }

    fn next_2_steps(&self, contract: NodeMetricsHistoryArgs) -> Vec<NodeMetricsHistoryResponse> {
        self.next(2, contract)
    }
}

async fn _daily_metrics_correct_different_update_size(size: usize) {
    let tracker = NodeMetricsHistoryResponseTracker::new()
        .with_subnet(subnet_id(1))
        .add_node_metrics(node_id(1), vec![(0, vec![(7, 5), (10, 6), (15, 6), (25, 50), (10, 6)])]);

    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .returning(move |contract| CallResult::Ok((tracker.next(size, contract),)));
    let mm = MetricsManager::new(mock);

    for _ in 0..MAX_TIMES {
        mm.update_subnets_metrics(vec![subnet_id(1)]).await;
    }
    let node_1_daily_metrics: Vec<NodeMetricsDailyStored> = mm
        .subnets_metrics
        .borrow()
        .values()
        .map(|node_metrics| node_metrics.0[0].clone())
        .collect();

    // (7, 5)
    assert_eq!(node_1_daily_metrics[0].num_blocks_proposed, 7);
    assert_eq!(node_1_daily_metrics[0].num_blocks_failed, 5);

    // (10 - 7, 6 - 5) = (3, 1)
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
async fn daily_metrics_correct_different_update_size() {
    _daily_metrics_correct_different_update_size(2).await;
    _daily_metrics_correct_different_update_size(3).await;
    _daily_metrics_correct_different_update_size(4).await;
    _daily_metrics_correct_different_update_size(5).await;
}

#[tokio::test]
async fn daily_metrics_correct_2_subs() {
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);

    let node_1 = node_id(1);

    let tracker = NodeMetricsHistoryResponseTracker::new()
        .with_subnet(subnet_1)
        .add_node_metrics(node_1, vec![(0, vec![(1, 1), (2, 2), (3, 3)])])
        .with_subnet(subnet_2)
        .add_node_metrics(node_1, vec![(3 * ONE_DAY_NANOS, vec![(4, 4), (6, 6), (8, 8)])]);

    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .returning(move |contract| CallResult::Ok((tracker.next_2_steps(contract),)));
    let mm = MetricsManager::new(mock);

    for _ in 0..MAX_TIMES {
        mm.update_subnets_metrics(vec![subnet_1, subnet_2]).await;
    }

    let node_1_daily_metrics = mm.daily_metrics_by_node(0, 8 * ONE_DAY_NANOS).get(&node_1).unwrap().clone();

    for (day, metrics) in node_1_daily_metrics.iter().enumerate() {
        match day {
            0 => {
                assert_eq!(metrics.subnet_assigned, subnet_1);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (1, 1));
            }
            1 => {
                assert_eq!(metrics.subnet_assigned, subnet_1);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (1, 1));
            }
            2 => {
                assert_eq!(metrics.subnet_assigned, subnet_1);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (1, 1));
            }
            3 => {
                assert_eq!(metrics.subnet_assigned, subnet_2);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (4, 4));
            }
            4 => {
                assert_eq!(metrics.subnet_assigned, subnet_2);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (2, 2));
            }
            _ => {
                assert_eq!(metrics.subnet_assigned, subnet_2);
                assert_eq!((metrics.num_blocks_proposed, metrics.num_blocks_failed), (2, 2));
            }
        }
    }
}

#[tokio::test]
async fn daily_metrics_correct_overlapping_days() {
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);

    let node_1 = node_id(1);
    let node_2 = node_id(1);

    let tracker = NodeMetricsHistoryResponseTracker::new()
        .with_subnet(subnet_1)
        .add_node_metrics(node_1, vec![(0, vec![(1, 1), (2, 2), (3, 3)])])
        .with_subnet(subnet_2)
        // Node 1 redeployed to subnet 2 on day 2
        .add_node_metrics(node_1, vec![(2 * ONE_DAY_NANOS, vec![(4, 4), (6, 6), (8, 8)])])
        .add_node_metrics(node_2, vec![(2 * ONE_DAY_NANOS, vec![(1, 1), (3, 3), (6, 6)])]);

    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .returning(move |contract| CallResult::Ok((tracker.next_2_steps(contract),)));
    let mm = MetricsManager::new(mock);

    for _ in 0..MAX_TIMES {
        mm.update_subnets_metrics(vec![subnet_id(1), subnet_id(2)]).await;
    }

    let daily_metrics = mm.daily_metrics_by_node(0, 4 * ONE_DAY_NANOS).get(&node_1).unwrap().clone();

    let overlapping_sub_1 = daily_metrics
        .iter()
        .find(|daily_metrics| daily_metrics.subnet_assigned == subnet_1 && daily_metrics.ts == DayEndNanos::from(2 * ONE_DAY_NANOS))
        .unwrap();

    assert_eq!(overlapping_sub_1.num_blocks_proposed, 1);
    assert_eq!(overlapping_sub_1.num_blocks_failed, 1);

    let overlapping_sub_2 = daily_metrics
        .iter()
        .find(|daily_metrics| daily_metrics.subnet_assigned == subnet_2 && daily_metrics.ts == DayEndNanos::from(2 * ONE_DAY_NANOS))
        .unwrap();

    assert_eq!(overlapping_sub_2.num_blocks_proposed, 4);
    assert_eq!(overlapping_sub_2.num_blocks_failed, 4);
}
