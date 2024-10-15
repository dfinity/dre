use std::path::PathBuf;

use decentralization::network::NodeFeaturePair;
use ic_management_types::NodeFeature;
use serde_yaml::Mapping;

use crate::store::Store;

fn ensure_empty(file: PathBuf) {
    std::fs::write(file, "").unwrap();
}

#[test]
fn fetch_from_git() {
    let offline = false;
    let store = Store::new(offline).unwrap();

    let cordoned_feature_fetcher = store.cordoned_features_fetcher().unwrap();
    ensure_empty(store.cordoned_features_file_outter().unwrap());

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let cordoned_features = runtime.block_on(cordoned_feature_fetcher.fetch()).unwrap();

    let cache_contents = std::fs::read_to_string(store.cordoned_features_file_outter().unwrap()).unwrap();
    let cordoned_features_from_cache = cordoned_feature_fetcher.parse_outter(cache_contents.as_bytes()).unwrap();

    assert_eq!(cordoned_features, cordoned_features_from_cache)
}

#[test]
fn fetch_offline_empty_cache() {
    let offline = true;
    let store = Store::new(offline).unwrap();

    let cordoned_feature_fetcher = store.cordoned_features_fetcher().unwrap();
    ensure_empty(store.cordoned_features_file_outter().unwrap());

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let cordoned_features = runtime.block_on(cordoned_feature_fetcher.fetch());

    assert!(cordoned_features.is_err())
}

fn write_to_cache(contents: Vec<NodeFeaturePair>, path: PathBuf) {
    let mut mapping = Mapping::new();
    mapping.insert("features".into(), serde_yaml::to_value(contents).unwrap());
    let root = serde_yaml::Value::Mapping(mapping);

    std::fs::write(path, serde_yaml::to_string(&root).unwrap()).unwrap()
}

#[test]
fn fetch_offline_cache() {
    let offline = true;
    let store = Store::new(offline).unwrap();

    let cordoned_feature_fetcher = store.cordoned_features_fetcher().unwrap();
    ensure_empty(store.cordoned_features_file_outter().unwrap());
    write_to_cache(
        vec![NodeFeaturePair {
            feature: NodeFeature::NodeProvider,
            value: "some-np".to_string(),
        }],
        store.cordoned_features_file_outter().unwrap(),
    );

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let cordoned_features = runtime.block_on(cordoned_feature_fetcher.fetch()).unwrap();

    let cache_contents = std::fs::read_to_string(store.cordoned_features_file_outter().unwrap()).unwrap();
    let cordoned_features_from_cache = cordoned_feature_fetcher.parse_outter(cache_contents.as_bytes()).unwrap();

    assert_eq!(cordoned_features, cordoned_features_from_cache)
}
