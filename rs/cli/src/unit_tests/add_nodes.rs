use std::sync::Arc;

use ic_base_types::PrincipalId;
use indexmap::IndexMap;

use ic_management_backend::{health::MockHealthStatusQuerier, lazy_registry::MockLazyRegistry};
use ic_management_types::{HealthStatus, Node, Operator, Provider};

use crate::subnet_manager::SubnetManager;

fn test_node(id: u64) -> Node {
    let principal = PrincipalId::new_node_test_id(id);
    Node {
        principal,
        ip_addr: None,
        operator: Operator {
            principal,
            provider: Provider {
                principal,
                name: None,
                website: None,
            },
            node_allowance: 1,
            datacenter: None,
            rewardable_nodes: Default::default(),
            max_rewardable_nodes: Default::default(),
            ipv6: "".to_string(),
        },
        cached_features: Default::default(),
        hostname: None,
        hostos_release: None,
        proposal: None,
        label: None,
        duplicates: None,
        subnet_id: None,
        hostos_version: String::new(),
        dfinity_owned: None,
        is_api_boundary_node: false,
        chip_id: None,
        public_ipv4_config: None,
        node_reward_type: None,
    }
}

#[test]
fn validate_add_nodes_success_and_failure() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Build world: two available nodes (1,2). Node 1 healthy, node 2 unhealthy; node 3 not available
    let available_nodes = vec![test_node(1), test_node(2)];
    let all_nodes_map: IndexMap<_, _> = available_nodes.iter().map(|n| (n.principal, n.clone())).collect();

    let mut registry = MockLazyRegistry::new();
    registry.expect_available_nodes().returning({
        let available_nodes = available_nodes.clone();
        move || {
            let available_nodes = available_nodes.clone();
            Box::pin(async move { Ok(available_nodes) })
        }
    });
    registry.expect_nodes().returning({
        let all_nodes_map = Arc::new(all_nodes_map.clone());
        move || {
            let all_nodes_map = all_nodes_map.clone();
            Box::pin(async move { Ok(all_nodes_map.clone()) })
        }
    });

    let mut health = MockHealthStatusQuerier::new();
    let mut healths: IndexMap<PrincipalId, HealthStatus> = IndexMap::new();
    healths.insert(PrincipalId::new_node_test_id(1), HealthStatus::Healthy);
    healths.insert(PrincipalId::new_node_test_id(2), HealthStatus::Dead);
    health.expect_nodes().returning({
        let healths = healths.clone();
        move || {
            let healths = healths.clone();
            Box::pin(async move { Ok(healths) })
        }
    });

    // SubnetManager with mocks
    let manager = SubnetManager::new(
        Arc::new(registry),
        Arc::new(crate::cordoned_feature_fetcher::MockCordonedFeatureFetcher::new()),
        Arc::new(health),
    );

    // Success: add-nodes contains only node 1
    let ok = runtime.block_on(manager.validate_nodes_available_and_healthy(&[PrincipalId::new_node_test_id(1)]));
    assert!(ok.is_ok(), "Expected node 1 to pass validation");

    // Failure: add-nodes contains node 2 (unhealthy) and 3 (not available)
    let err = runtime.block_on(manager.validate_nodes_available_and_healthy(&[PrincipalId::new_node_test_id(2), PrincipalId::new_node_test_id(3)]));
    assert!(err.is_err(), "Expected validation to fail for nodes 2 and 3");
    let msg = format!("{}", err.unwrap_err());
    assert!(msg.contains("not available"), "Missing not available reason: {}", msg);
    assert!(msg.contains("not healthy"), "Missing not healthy reason: {}", msg);
}
