use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr};

use ic_canisters::governance::governance_canister_version;
use ic_management_backend::{
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};

use crate::{auth::Neuron, commands::Args};

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    registry: Option<RegistryState>,
    neuron: Option<Neuron>,
    // ic-admin wrapper
}

impl DreContext {
    pub async fn from_args(args: Args, require_neuron: bool, require_registry: bool) -> anyhow::Result<Self> {
        let network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let nns_urls = network.get_nns_urls();

        let (neuron_id, private_key_pem) = {
            let path = PathBuf::from_str(&std::env::var("HOME")?)?.join("/.config/dfx/identity/bootstrap-super-leader/identity.pem");
            match network.name.as_str() {
                "staging" if path.exists() => (Some(STAGING_NEURON_ID), Some(path)),
                "staging" => (Some(STAGING_NEURON_ID), args.private_key_pem),
                _ => (args.neuron_id, args.private_key_pem),
            }
        };

        let neuron = match require_neuron {
            true => Some(Neuron::new(private_key_pem, args.hsm_slot, args.hsm_pin, args.hsm_key_id, neuron_id, &network).await?),
            false => None,
        };

        let registry = match require_registry {
            true => Some(Self::init_registry(&network).await?),
            false => None,
        };

        let govn_canister_version = governance_canister_version(nns_urls).await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(Self { network, registry, neuron })
    }

    async fn init_registry(network: &Network) -> anyhow::Result<RegistryState> {
        // Create a new registry state
        let mut new_registry = RegistryState::new(network, true).await;

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(network, "v3/node-providers")
            .await?
            .node_providers;

        fetch_and_add_node_labels_guests_to_registry(network, &mut new_registry).await;

        // Update node details
        new_registry.update_node_details(&node_providers).await?;

        Ok(new_registry)
    }
}
