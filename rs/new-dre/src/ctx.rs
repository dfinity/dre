use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr};

use ic_canisters::governance::governance_canister_version;
use ic_management_backend::{
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};

use crate::commands::Args;

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    private_key_pem: Option<PathBuf>,
    neuron: Option<u64>,
    governance_canister_version_hash: String,
    registry: RefCell<Option<Rc<RegistryState>>>,
}

impl DreContext {
    pub async fn from_args(args: Args) -> anyhow::Result<Self> {
        let network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let nns_urls = network.get_nns_urls();

        let private_key_pem = match args.private_key_pem {
            Some(path) => Some(PathBuf::from(path)),
            None if network.name == "staging" => {
                let path = PathBuf::from_str(&std::env::var("HOME")?)?.join("/.config/dfx/identity/bootstrap-super-leader/identity.pem");
                match path.exists() {
                    true => Some(path),
                    false => None,
                }
            }
            None => None,
        };

        let govn_canister_version = governance_canister_version(nns_urls).await.map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            network,
            private_key_pem,
            neuron: args.neuron_id,
            governance_canister_version_hash: govn_canister_version.stringified_hash,
            registry: RefCell::new(None),
        })
    }

    pub async fn get_registry(&self) -> anyhow::Result<Rc<RegistryState>> {
        if let Some(ref registry) = *self.registry.borrow() {
            return Ok(registry.clone());
        }

        // Create a new registry state
        let mut new_registry = RegistryState::new(&self.network, true).await;

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers")
            .await?
            .node_providers;

        fetch_and_add_node_labels_guests_to_registry(&self.network, &mut new_registry).await;

        // Update node details
        new_registry.update_node_details(&node_providers).await?;

        let new_registry_rc = Rc::from(new_registry);
        // Replace the registry in self with the new registry state
        self.registry.replace(Some(new_registry_rc.clone()));

        // Return the Rc to the new registry state
        Ok(new_registry_rc)
    }
}
