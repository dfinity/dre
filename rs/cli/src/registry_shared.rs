use std::{cell::RefCell, rc::Rc};

use ic_management_backend::{
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};

pub struct Registry {
    registry: RefCell<Option<Rc<RegistryState>>>,
    network: Network,
}

impl Registry {
    pub fn new(network: &Network) -> Rc<Self> {
        Rc::from(Self {
            registry: RefCell::new(None),
            network: network.clone(),
        })
    }

    pub async fn get(&self) -> Rc<RegistryState> {
        {
            if let Some(ref registry) = *self.registry.borrow() {
                return registry.clone();
            }
        }

        // Create a new registry state
        let mut new_registry = RegistryState::new(&self.network, true).await;

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers")
            .await
            .expect("Failed to get node providers")
            .node_providers;

        fetch_and_add_node_labels_guests_to_registry(&self.network, &mut new_registry).await;

        // Update node details
        new_registry
            .update_node_details(&node_providers)
            .await
            .expect("Failed to update node details");

        let new_registry_rc = Rc::from(new_registry);
        // Replace the registry in self with the new registry state
        self.registry.replace(Some(new_registry_rc.clone()));

        // Return the Rc to the new registry state
        new_registry_rc
    }
}
