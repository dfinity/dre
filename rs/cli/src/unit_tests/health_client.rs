use ic_management_backend::health::ShortNodeInfo;
use ic_types::PrincipalId;

use crate::store::Store;

#[derive(Debug)]
struct TestScenario {
    name: String,
    network: String,
    offline: bool,
    cache: Option<Vec<ShortNodeInfo>>,
    should_succeed: bool,
}

impl TestScenario {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cache: None,
            network: "mainnet".to_string(),
            offline: false,
            should_succeed: true,
        }
    }

    fn online(self) -> Self {
        Self { offline: false, ..self }
    }

    fn offline(self) -> Self {
        Self { offline: true, ..self }
    }

    fn with_network(self, network: &str) -> Self {
        Self {
            network: network.to_string(),
            ..self
        }
    }

    fn with_cache(self, cache: &[ShortNodeInfo]) -> Self {
        Self {
            cache: Some(cache.to_vec()),
            ..self
        }
    }

    fn no_cache(self) -> Self {
        Self { cache: None, ..self }
    }

    fn should_succeed(self) -> Self {
        Self {
            should_succeed: true,
            ..self
        }
    }

    fn should_fail(self) -> Self {
        Self {
            should_succeed: false,
            ..self
        }
    }
}

#[test]
fn health_client_tests() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let scenarios = vec![
        TestScenario::new("[Online] Should fetch and update node healths")
            .online()
            .with_network("mainnet")
            .no_cache()
            .should_succeed(),
        TestScenario::new("[Online] Should update stale cache")
            .online()
            .with_network("mainnet")
            .with_cache(&[ShortNodeInfo {
                node_id: PrincipalId::new_node_test_id(1),
                subnet_id: None,
                status: ic_management_types::HealthStatus::Healthy,
            }])
            .should_succeed(),
        TestScenario::new("[Offline] No cache").offline().no_cache().should_fail(),
        TestScenario::new("[Offline] Should fetch from cache")
            .offline()
            .with_cache(&[ShortNodeInfo {
                node_id: PrincipalId::new_node_test_id(1),
                subnet_id: None,
                status: ic_management_types::HealthStatus::Healthy,
            }])
            .should_succeed(),
    ];

    let mut failed_scenarios = vec![];
    for scenario in &scenarios {
        println!("### Starting scenario `{}`", scenario.name);
        let store = Store::new(scenario.offline).unwrap();
        let network = ic_management_types::Network::new_unchecked(scenario.network.clone(), &[]).unwrap();
        let cache_path = store.node_health_file_outer(&network).unwrap();

        match &scenario.cache {
            Some(cache) => std::fs::write(&cache_path, serde_json::to_string_pretty(cache).unwrap()).unwrap(),
            None => std::fs::write(&cache_path, "").unwrap(),
        }

        let health_client = store.health_client(&network).unwrap();

        let maybe_nodes_info = runtime.block_on(health_client.nodes_short_info());

        if scenario.should_succeed != maybe_nodes_info.is_ok() {
            failed_scenarios.push((maybe_nodes_info, scenario));
            println!("### Ending scenario `{}`", scenario.name);
            continue;
        }

        // Scenario should fail but it is expected to fail
        if !scenario.should_succeed {
            println!("### Ending scenario `{}`", scenario.name);
            continue;
        }

        let nodes_info = maybe_nodes_info.unwrap();
        let contents = std::fs::read_to_string(&cache_path).unwrap();
        let cached: Vec<ShortNodeInfo> = serde_json::from_str(&contents).unwrap();

        if !PartialEq::eq(&cached, &nodes_info) {
            failed_scenarios.push((Ok(nodes_info), scenario));
        }

        println!("### Ending scenario `{}`", scenario.name);
    }
}
