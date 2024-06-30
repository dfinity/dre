use std::{cell::RefCell, rc::Rc};

use ic_management_backend::{public_dashboard::query_ic_dashboard_list, registry::RegistryState};
use ic_management_types::{Network, NodeProvidersResponse};

pub struct RegistryShared {
    registry: RefCell<Option<Rc<RegistryState>>>,
    network: Network,
}

impl RegistryShared {
    pub fn new(network: &Network) -> Rc<Self> {
        Rc::from(Self {
            registry: RefCell::new(None),
            network: network.clone(),
        })
    }

    pub async fn registry(&self) -> Rc<RegistryState> {
        {
            if let Some(ref registry) = *self.registry.borrow() {
                return registry.clone()
            }
        }

        // Create a new registry state
        let mut new_registry = RegistryState::new(&self.network, true).await;

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers")
            .await
            .expect("Failed to get node providers")
            .node_providers;

        // Update node details
        new_registry
            .update_node_details(&node_providers)
            .await
            .expect("Failed to update node details");

        let new_registry_rc = Rc::from(new_registry);
        // Replace the registry in self with the new registry state
        self.registry.replace(Some(new_registry_rc.clone()));

        // Return the Arc to the new registry state
        new_registry_rc
    }
}
