use std::path::PathBuf;

use decentralization::network::CordonedFeature;
use ic_management_types::NodeFeature;
use itertools::Itertools;
use serde_yaml::Mapping;

use crate::store::Store;

fn ensure_empty(file: PathBuf) {
    fs_err::write(file, "").unwrap();
}

fn write_to_cache(contents: &[CordonedFeature], path: PathBuf) {
    let mut mapping = Mapping::new();
    mapping.insert("features".into(), serde_yaml::to_value(contents).unwrap());
    let root = serde_yaml::Value::Mapping(mapping);

    fs_err::write(path, serde_yaml::to_string(&root).unwrap()).unwrap()
}

#[derive(Debug)]
struct TestScenario {
    name: String,
    offline: bool,
    cache_contents: Option<Vec<CordonedFeature>>,
    should_succeed: bool,
}

impl TestScenario {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            offline: false,
            cache_contents: None,
            should_succeed: false,
        }
    }

    fn offline(self) -> Self {
        Self { offline: true, ..self }
    }

    fn online(self) -> Self {
        Self { offline: false, ..self }
    }

    fn no_cache(self) -> Self {
        Self {
            cache_contents: None,
            ..self
        }
    }

    fn with_cache(self, pairs: &[CordonedFeature]) -> Self {
        Self {
            cache_contents: Some(pairs.to_vec()),
            ..self
        }
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
fn cordoned_feature_fetcher_tests() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let scenarios = vec![
        TestScenario::new("[Online] Fetch from git").no_cache().online().should_succeed(),
        TestScenario::new("[Offline] No cache").no_cache().offline().should_fail(),
        TestScenario::new("[Offline] Fetch from cache")
            .offline()
            .with_cache(&[CordonedFeature {
                feature: NodeFeature::NodeProvider,
                value: "some-np".to_string(),
                explanation: None,
            }])
            .should_succeed(),
        TestScenario::new("[Online] Stale cache")
            .online()
            .with_cache(&[CordonedFeature {
                feature: NodeFeature::NodeProvider,
                value: "some-np".to_string(),
                explanation: None,
            }])
            .should_succeed(),
    ];

    let mut failed_scenarios = vec![];

    for scenario in &scenarios {
        println!("### Starting scenario `{}`", scenario.name);
        let store = Store::new(scenario.offline).unwrap();

        match &scenario.cache_contents {
            Some(cache) => write_to_cache(cache, store.cordoned_features_file_outer().unwrap()),
            None => ensure_empty(store.cordoned_features_file_outer().unwrap()),
        }

        let cordoned_feature_fetcher = store.cordoned_features_fetcher(None).unwrap();

        let maybe_cordoned_features = runtime.block_on(cordoned_feature_fetcher.fetch());

        if !scenario.should_succeed {
            if let Ok(features) = maybe_cordoned_features {
                failed_scenarios.push((features, scenario));
            }
            println!("### Ending scenario `{}`", scenario.name);
            continue;
        }

        let cordoned_features = maybe_cordoned_features.unwrap();
        let cache_contents = fs_err::read_to_string(store.cordoned_features_file_outer().unwrap()).unwrap();
        let cordoned_features_from_cache = cordoned_feature_fetcher.parse_outer(cache_contents.as_bytes()).unwrap();

        if !cordoned_features.eq(&cordoned_features_from_cache) {
            failed_scenarios.push((cordoned_features, scenario));
        }
        println!("### Ending scenario `{}`", scenario.name);
    }

    assert!(
        failed_scenarios.is_empty(),
        "Failed scenarios:\n{}",
        failed_scenarios
            .iter()
            .map(|(features, scenario)| format!("\tScenario: {:?}\n\tGot Features: {:?}", scenario, features))
            .join("\n")
    )
}
