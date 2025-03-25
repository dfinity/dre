use crate::metrics::MetricsManager;
use crate::metrics_types::StorableNodeMetricsKey;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_cdk::api::call::{CallResult, RejectionCode};
use ic_management_canister_types::{NodeMetrics, NodeMetricsHistoryResponse};
use ic_stable_structures::memory_manager::{MemoryId, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use std::cell::RefCell;

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
            async fn node_metrics_history(&self, contract: ic_management_canister_types::NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
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
            nodes_metrics: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(0))),
            subnets_to_retry: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(1))),
            last_timestamp_per_subnet: RefCell::new(crate::storage::stable_btreemap_init(MemoryId::new(2))),
        }
    }
}

fn node_metrics_history_gen(days: u64, nodes: Vec<NodeId>) -> Vec<NodeMetricsHistoryResponse> {
    let mut result = Vec::new();
    for i in 0..days {
        let node_metrics = nodes
            .iter()
            .map(|node| NodeMetrics {
                node_id: node.get(),
                ..Default::default()
            })
            .collect();
        result.push(NodeMetricsHistoryResponse {
            timestamp_nanos: i * ONE_DAY_NANOS,
            node_metrics,
        });
    }
    result
}

#[tokio::test]
async fn subnet_metrics_added_correctly() {
    let days = 45;
    let node_1 = node_id(1);
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days, vec![node_1]),)));
    let mm = MetricsManager::new(mock);

    let subnet_1 = subnet_id(1);

    mm.update_subnets_metrics(vec![subnet_1]).await;
    for i in 0..days {
        let key = StorableNodeMetricsKey {
            ts: i * ONE_DAY_NANOS,
            node_id: node_1,
            subnet_assigned: subnet_1,
        };
        assert!(mm.nodes_metrics.borrow().get(&key).is_some());
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
        .return_const(CallResult::Ok((node_metrics_history_gen(2, vec![]),)));

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
    let node_1 = node_id(1);

    let mut mock = mock::MockCanisterClient::new();

    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days, vec![node_1]),)));
    let mm = MetricsManager::new(mock);
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);

    mm.update_subnets_metrics(vec![subnet_1, subnet_2]).await;

    for subnet in &[subnet_1, subnet_2] {
        for i in 0..days {
            let key = StorableNodeMetricsKey {
                ts: i * ONE_DAY_NANOS,
                node_id: node_1,
                subnet_assigned: *subnet,
            };
            assert!(mm.nodes_metrics.borrow().get(&key).is_some(), "Metrics missing for subnet {:?}", subnet);
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

    assert!(mm.nodes_metrics.borrow().is_empty(), "Metrics should be empty after a failed call");
}

#[tokio::test]
async fn partial_failures_are_handled_correctly() {
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);
    let node_1 = node_id(1);

    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history().returning(move |subnet| {
        if SubnetId::from(subnet.subnet_id) == subnet_1 {
            CallResult::Err((RejectionCode::Unknown, "Error".to_string()))
        } else {
            CallResult::Ok((node_metrics_history_gen(1, vec![node_1]),))
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

    let key = StorableNodeMetricsKey {
        ts: 0,
        node_id: node_1,
        subnet_assigned: subnet_1,
    };
    assert!(
        mm.nodes_metrics.borrow().get(&key).is_none(),
        "Metrics should not be present for subnet 1"
    );

    let key = StorableNodeMetricsKey {
        ts: 0,
        node_id: node_1,
        subnet_assigned: subnet_2,
    };
    assert!(mm.nodes_metrics.borrow().get(&key).is_some(), "Metrics should be present for subnet 2");
}
