use crate::metrics::MetricsManager;
use crate::metrics_types::StorableSubnetMetricsKey;
use ic_base_types::{PrincipalId, SubnetId};
use ic_cdk::api::call::{CallResult, RejectionCode};
use ic_management_canister_types::NodeMetricsHistoryResponse;
use ic_stable_structures::memory_manager::{MemoryId, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use std::cell::RefCell;
use std::rc::Rc;

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

impl MetricsManager<VM> {
    fn new(client: mock::MockCanisterClient) -> Self {
        Self {
            client: Rc::new(client),
            subnets_metrics: crate::storage::stable_btreemap_init(MemoryId::new(1)),
            subnets_to_retry: crate::storage::stable_btreemap_init(MemoryId::new(2)),
            last_timestamp_per_subnet: crate::storage::stable_btreemap_init(MemoryId::new(3)),
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
    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));

    let subnet_1 = subnet_id(1);

    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1]).await;
    for i in 0..days {
        let key = StorableSubnetMetricsKey {
            timestamp_nanos: i * ONE_DAY_NANOS,
            subnet_id: subnet_1,
        };
        assert!(mm_rc.borrow().subnets_metrics.get(&key).is_some());
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

    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));
    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1]).await;
    assert_eq!(mm_rc.borrow().subnets_to_retry.get(&subnet_1.into()), Some(1));

    // Retry the subnet and success
    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1]).await;
    assert_eq!(mm_rc.borrow().subnets_to_retry.get(&subnet_1.into()), None);
}

#[tokio::test]
async fn multiple_subnets_metrics_added_correctly() {
    let days = 30;
    let mut mock = mock::MockCanisterClient::new();

    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days),)));
    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);

    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1, subnet_2]).await;

    println!("{:?}", mm_rc.borrow().subnets_metrics.len());

    for subnet in &[subnet_1, subnet_2] {
        for i in 0..days {
            let key = StorableSubnetMetricsKey {
                timestamp_nanos: i * ONE_DAY_NANOS,
                subnet_id: *subnet,
            };
            assert!(
                mm_rc.borrow().subnets_metrics.get(&key).is_some(),
                "Metrics missing for subnet {:?}",
                subnet
            );
        }
    }
}

#[tokio::test]
async fn retry_count_increments_on_failure() {
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .return_const(CallResult::Err((RejectionCode::Unknown, "Temporary error".to_string())));

    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));
    let subnet_1 = subnet_id(1);

    for retry_attempt in 1..=3 {
        MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1]).await;
        assert_eq!(
            mm_rc.borrow().subnets_to_retry.get(&subnet_1.into()),
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
    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));

    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1]).await;

    assert!(mm_rc.borrow().subnets_metrics.is_empty(), "Metrics should be empty after a failed call");
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

    let mm_rc = Rc::new(RefCell::new(MetricsManager::new(mock)));

    MetricsManager::update_subnets_metrics(mm_rc.clone(), vec![subnet_1, subnet_2]).await;

    assert_eq!(
        mm_rc.borrow().subnets_to_retry.get(&subnet_1.into()),
        Some(1),
        "Subnet 1 should be in retry list"
    );
    assert!(
        mm_rc.borrow().subnets_to_retry.get(&subnet_2.into()).is_none(),
        "Subnet 2 should not be in retry list"
    );

    let key = StorableSubnetMetricsKey {
        timestamp_nanos: 0,
        subnet_id: subnet_1,
    };
    assert!(
        mm_rc.borrow().subnets_metrics.get(&key).is_none(),
        "Metrics should not be present for subnet 1"
    );

    let key = StorableSubnetMetricsKey {
        timestamp_nanos: 0,
        subnet_id: subnet_2,
    };
    assert!(
        mm_rc.borrow().subnets_metrics.get(&key).is_some(),
        "Metrics should be present for subnet 2"
    );
}
