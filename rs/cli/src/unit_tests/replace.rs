use std::sync::Arc;

use decentralization::{
    nakamoto::NodeFeatures,
    network::{DecentralizedSubnet, Node, NodeFeaturePair},
};
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_registry::MockLazyRegistry};
use ic_management_types::NodeFeature;
use ic_types::PrincipalId;
use indexmap::IndexMap;

use crate::{cordoned_feature_fetcher::MockCordonedFeatureFetcher, subnet_manager::SubnetManager};

#[test]
fn should_skip_cordoned_nodes() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut registry = MockLazyRegistry::new();
    registry.expect_available_nodes().returning(|| {
        Box::pin(async {
            Ok(vec![Node {
                dfinity_owned: true,
                features: NodeFeatures {
                    feature_map: {
                        let mut map = IndexMap::new();

                        map.insert(NodeFeature::Country, "Cambodia".to_string());

                        map
                    },
                },
                id: PrincipalId::new_node_test_id(1),
            }])
        })
    });
    registry.expect_subnet().returning(|_| {
        Box::pin(async {
            Ok(DecentralizedSubnet {
                id: PrincipalId::new_subnet_test_id(1),
                nodes: vec![Node {
                    dfinity_owned: true,
                    features: NodeFeatures {
                        feature_map: {
                            let mut map = IndexMap::new();

                            map.insert(NodeFeature::Country, "This".to_string());

                            map
                        },
                    },
                    id: PrincipalId::new_node_test_id(2),
                }],
                added_nodes_desc: vec![],
                removed_nodes_desc: vec![],
                comment: None,
                run_log: vec![],
            })
        })
    });

    let mut health_client = MockHealthStatusQuerier::new();
    health_client.expect_nodes().returning(|| {
        Box::pin(async {
            Ok({
                let mut nodes = IndexMap::new();
                nodes.insert(PrincipalId::new_node_test_id(1), ic_management_types::HealthStatus::Healthy);
                nodes
            })
        })
    });
    let mut cordoned_feature_fetcher = MockCordonedFeatureFetcher::new();
    cordoned_feature_fetcher.expect_fetch().returning(|| {
        Box::pin(async {
            Ok(vec![NodeFeaturePair {
                feature: NodeFeature::Country,
                value: "Other".to_string(),
            }])
        })
    });
    let subnet_manager = SubnetManager::new(Arc::new(registry), Arc::new(cordoned_feature_fetcher), Arc::new(health_client));

    let response = runtime.block_on(
        subnet_manager
            .with_target(crate::subnet_manager::SubnetTarget::FromId(PrincipalId::new_subnet_test_id(1)))
            .membership_replace(false, None, None, None, vec![], None),
    );
    assert!(response.is_ok());

    let response = response.unwrap();
}
