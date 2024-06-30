use std::{cell::RefCell, sync::Arc};

use ic_management_backend::{public_dashboard::query_ic_dashboard_list, registry::RegistryState};
use ic_management_types::{Network, NodeProvidersResponse};

pub struct RegistryShared {
    registry: RefCell<Option<Arc<RegistryState>>>,
    network: Network,
}

impl RegistryShared {
    pub fn new(network: &Network) -> Self {
        Self {
            registry: RefCell::new(None),
            network: network.clone(),
        }
    }
}

impl RegistryShared{
    pub async fn registry(&self) -> Arc<RegistryState> {
        {
            if let Some(ref registry) = *self.registry.borrow() {
                return Arc::clone(registry);
            }
        }

        // Create a new registry state
        let mut new_registry = Arc::new(RegistryState::new(&self.network, true).await);

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers")
            .await
            .expect("Failed to get node providers")
            .node_providers;

        // Update node details
        Arc::get_mut(&mut new_registry)
            .expect("Failed to get mutable reference to new registry")
            .update_node_details(&node_providers)
            .await
            .expect("Failed to update node details");

        // Replace the registry in self with the new registry state
        self.registry.replace(Some(Arc::clone(&new_registry)));

        // Return the Arc to the new registry state
        new_registry
    }
}

#[allow(async_fn_in_trait)]
pub trait RegistryGetter {
    async fn registry(&self) -> Arc<RegistryState>;
}
