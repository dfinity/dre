use std::sync::Arc;

use ic_management_backend::{health::MockHealthStatusQuerier, lazy_registry::MockLazyRegistry};

use crate::{cordoned_feature_fetcher::MockCordonedFeatureFetcher, subnet_manager::SubnetManager};

#[test]
fn should_skip_cordoned_nodes() {
    let registry = MockLazyRegistry::new();
    let health_client = MockHealthStatusQuerier::new();
    let cordoned_feature_fetcher = MockCordonedFeatureFetcher::new();
    let subnet_manager = SubnetManager::new(Arc::new(registry), Arc::new(cordoned_feature_fetcher), Arc::new(health_client));
}
