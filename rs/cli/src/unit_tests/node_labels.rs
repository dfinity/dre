use std::path::PathBuf;

use ic_management_backend::node_labels;
use ic_management_types::{Guest, Network};
use itertools::Itertools;

use crate::store::Store;

fn ensure_empty(path: PathBuf) {
    fs_err::write(path, "").unwrap()
}

fn write_cache(guests: &[Guest], path: PathBuf) {
    let mut v1 = serde_yaml::Mapping::new();
    for guest in guests {
        let mut current = serde_yaml::Mapping::new();
        current.insert("dc".into(), guest.datacenter.clone().into());
        current.insert("label".into(), guest.name.clone().into());
        v1.insert(guest.ipv6.to_string().into(), current.into());
    }

    let mut data = serde_yaml::Mapping::new();
    data.insert("v1".into(), v1.into());
    let mut root = serde_yaml::Mapping::new();
    root.insert("data".into(), data.into());

    fs_err::write(path, serde_yaml::to_string(&root).unwrap()).unwrap()
}

#[derive(Debug)]
struct TestScenario {
    name: String,
    network: String,
    local_cache: Option<Vec<Guest>>,
    offline: bool,
    should_succeed: bool,
}

impl TestScenario {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            local_cache: None,
            network: "mainnet".to_string(),
            offline: false,
            should_succeed: true,
        }
    }

    fn no_cache(self) -> Self {
        Self { local_cache: None, ..self }
    }

    fn with_cache(self, guests: &[Guest]) -> Self {
        Self {
            local_cache: Some(guests.to_vec()),
            ..self
        }
    }

    fn with_network(self, network: &str) -> Self {
        Self {
            network: network.to_string(),
            ..self
        }
    }

    fn offline(self) -> Self {
        Self { offline: true, ..self }
    }

    fn online(self) -> Self {
        Self { offline: false, ..self }
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
fn test_node_labels() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let scenarios = vec![
        TestScenario::new("[Offline] No local cache")
            .with_network("mainnet")
            .offline()
            .no_cache()
            .should_fail(),
        TestScenario::new("[Online] Should update empty cache")
            .with_network("mainnet")
            .online()
            .no_cache(),
        TestScenario::new("[Online] Should update empty cache staging")
            .with_network("staging")
            .online()
            .no_cache(),
        TestScenario::new("[Offline] Should read from cache")
            .offline()
            .with_network("mainnet")
            .with_cache(&[Guest {
                datacenter: "test-dc".to_string(),
                ipv6: "::1".parse().unwrap(),
                name: "test-label".to_string(),
                dfinity_owned: false,
            }])
            .should_succeed(),
    ];

    let mut failed_scenarios = vec![];

    for scenario in &scenarios {
        eprintln!("### Starting scenario `{}`", scenario.name);
        let network = Network::new_unchecked(scenario.network.clone(), &[]).unwrap();
        let store = Store::new(scenario.offline).unwrap();
        let labels_path = store.guest_labels_cache_path_outer(&network).unwrap();

        match &scenario.local_cache {
            Some(guests) => write_cache(guests, labels_path.clone()),
            None => ensure_empty(labels_path.clone()),
        }

        let maybe_labels = runtime.block_on(node_labels::query_guests(
            &scenario.network,
            Some(labels_path.clone()),
            store.is_offline(),
        ));

        if !scenario.should_succeed {
            if let Ok(labels) = maybe_labels {
                failed_scenarios.push((labels, scenario));
            }
            eprintln!("### Ending scenario `{}`", scenario.name);
            continue;
        }

        let labels = maybe_labels.unwrap();
        let content = fs_err::read_to_string(labels_path.clone()).unwrap();
        let labels_from_cache = node_labels::parse_data(content).unwrap();
        if !labels.eq(&labels_from_cache) {
            failed_scenarios.push((labels, scenario));
        }
        eprintln!("### Ending scenario `{}`", scenario.name);
    }

    assert!(
        failed_scenarios.is_empty(),
        "Failed scenarios:\n{}",
        failed_scenarios
            .iter()
            .map(|(labels, scenario)| format!("\tScenario: {:?}\n\tGot Labels: {:?}", scenario, labels))
            .join("\n")
    )
}
