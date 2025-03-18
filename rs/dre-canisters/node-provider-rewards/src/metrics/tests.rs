use crate::metrics::MetricsManagerData;
use crate::metrics_types::StorableSubnetMetricsKey;
use crate::storage::State;
use crate::MetricsManagerInstance;
use ic_base_types::{PrincipalId, SubnetId};
use ic_cdk::api::call::{CallResult, RejectionCode};
use ic_management_canister_types::NodeMetricsHistoryResponse;

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

const ONE_DAY_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
fn subnet_id(id: u64) -> ic_base_types::SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
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
    let mut mock = mock::MockCanisterClient::new();
    let days = 45;
    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days),)));
    let subnet_1 = subnet_id(1);

    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1]).await;
    State::with_subnets_metrics(|subnets_metrics| {
        for i in 0..days {
            let key = StorableSubnetMetricsKey {
                timestamp_nanos: i * ONE_DAY_NANOS,
                subnet_id: subnet_1,
            };
            assert!(subnets_metrics.get(&key).is_some());
        }
    });
}

#[tokio::test]
async fn subnets_to_retry_filled() {
    let subnet_1 = subnet_id(1);
    let mut mock = mock::MockCanisterClient::new();
    mock.expect_node_metrics_history()
        .times(1)
        .return_const(CallResult::Err((RejectionCode::Unknown, "Error".to_string())));

    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1]).await;
    State::with_subnets_to_retry(|subnets_to_retry| assert_eq!(subnets_to_retry.get(&subnet_1.into()), Some(1)));

    // Retry the subnet and success
    mock.expect_node_metrics_history()
        .times(1)
        .return_const(CallResult::Ok((node_metrics_history_gen(2),)));
    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1]).await;
    State::with_subnets_to_retry(|subnets_to_retry| assert_eq!(subnets_to_retry.get(&subnet_1.into()), None));
}

#[tokio::test]
async fn multiple_subnets_metrics_added_correctly() {
    let mut mock = mock::MockCanisterClient::new();
    let subnet_1 = subnet_id(1);
    let subnet_2 = subnet_id(2);
    let days = 30;

    mock.expect_node_metrics_history()
        .return_const(CallResult::Ok((node_metrics_history_gen(days),)));

    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1, subnet_2]).await;

    State::with_subnets_metrics(|subnets_metrics| {
        for subnet in &[subnet_1, subnet_2] {
            for i in 0..days {
                let key = StorableSubnetMetricsKey {
                    timestamp_nanos: i * ONE_DAY_NANOS,
                    subnet_id: *subnet,
                };
                assert!(subnets_metrics.get(&key).is_some(), "Metrics missing for subnet {:?}", subnet);
            }
        }
    });
}

#[tokio::test]
async fn retry_count_increments_on_failure() {
    let subnet_1 = subnet_id(1);
    let mut mock = mock::MockCanisterClient::new();

    mock.expect_node_metrics_history()
        .return_const(CallResult::Err((RejectionCode::Unknown, "Temporary error".to_string())));

    for retry_attempt in 1..=3 {
        MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1]).await;
        State::with_subnets_to_retry(|subnets_to_retry| {
            assert_eq!(
                subnets_to_retry.get(&subnet_1.into()),
                Some(retry_attempt),
                "Retry count should be {}",
                retry_attempt
            );
        });
    }
}

#[tokio::test]
async fn no_metrics_added_when_call_fails() {
    let subnet_1 = subnet_id(1);
    let mut mock = mock::MockCanisterClient::new();

    mock.expect_node_metrics_history()
        .return_const(CallResult::Err((RejectionCode::Unknown, "Error".to_string())));

    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1]).await;

    State::with_subnets_metrics(|subnets_metrics| {
        assert!(subnets_metrics.is_empty(), "Metrics should be empty after a failed call");
    });
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

    MetricsManagerInstance::update_subnets_metrics(&mock, vec![subnet_1, subnet_2]).await;

    State::with_subnets_to_retry(|subnets_to_retry| {
        assert_eq!(subnets_to_retry.get(&subnet_1.into()), Some(1), "Subnet 1 should be in retry list");
        assert!(subnets_to_retry.get(&subnet_2.into()).is_none(), "Subnet 2 should not be in retry list");
    });

    State::with_subnets_metrics(|subnets_metrics| {
        let key = StorableSubnetMetricsKey {
            timestamp_nanos: 0,
            subnet_id: subnet_1,
        };
        assert!(subnets_metrics.get(&key).is_none(), "Metrics should not be present for subnet 1");

        let key = StorableSubnetMetricsKey {
            timestamp_nanos: 0,
            subnet_id: subnet_2,
        };
        assert!(subnets_metrics.get(&key).is_some(), "Metrics should be present for subnet 2");
    });
}
