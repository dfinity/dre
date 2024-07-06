use std::{path::PathBuf, rc::Rc, str::FromStr};

use ic_canisters::{governance::governance_canister_version, CanisterClient};
use ic_management_backend::{
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};

use crate::{
    auth::Neuron,
    commands::{Args, ExecutableCommand},
    ic_admin::{download_ic_admin, IcAdminWrapper},
    runner::Runner,
};

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    registry: Option<Rc<RegistryState>>,
    ic_admin: Option<Rc<IcAdminWrapper>>,
}

impl DreContext {
    pub async fn from_args(args: &Args) -> anyhow::Result<Self> {
        let network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let (neuron_id, private_key_pem) = {
            let path = PathBuf::from_str(&std::env::var("HOME")?)?.join("/.config/dfx/identity/bootstrap-super-leader/identity.pem");
            match network.name.as_str() {
                "staging" if path.exists() => (Some(STAGING_NEURON_ID), Some(path)),
                "staging" => (Some(STAGING_NEURON_ID), args.private_key_pem.clone()),
                _ => (args.neuron_id.clone(), args.private_key_pem.clone()),
            }
        };

        let ic_admin = match args.require_neuron() {
            true => Some(Rc::new(
                Self::init_ic_admin(
                    &network,
                    neuron_id,
                    private_key_pem,
                    args.hsm_slot,
                    args.hsm_key_id.clone(),
                    args.hsm_pin.clone(),
                    args.yes,
                )
                .await?,
            )),
            false => None,
        };

        let registry = match args.require_registry() {
            true => Some(Rc::new(Self::init_registry(&network).await?)),
            false => None,
        };

        Ok(Self { network, registry, ic_admin })
    }

    async fn init_ic_admin(
        network: &Network,
        neuron_id: Option<u64>,
        private_key_pem: Option<PathBuf>,
        hsm_slot: Option<u64>,
        hsm_key_id: Option<String>,
        hsm_pin: Option<String>,
        proceed_without_confirmation: bool,
    ) -> anyhow::Result<IcAdminWrapper> {
        let neuron = Neuron::new(private_key_pem, hsm_slot, hsm_pin.clone(), hsm_key_id.clone(), neuron_id, &network).await?;

        let govn_canister_version = governance_canister_version(network.get_nns_urls()).await?;
        let ic_admin_path = download_ic_admin(Some(govn_canister_version.stringified_hash)).await?;

        let ic_admin = IcAdminWrapper::new(network.clone(), Some(ic_admin_path), proceed_without_confirmation, neuron);

        Ok(ic_admin)
    }

    /// Here we can gain more startup speed if we find a way to optimize RegistryState struct
    ///
    /// We could refactor the RegistryState struct to have certain levels of information eg:
    /// 1. Raw (cotains just the LocalRegistry and one can only use that) - useful for registry command
    /// 2. Node details (contains the node_providers + the call to update_node_details) - most of the commands
    /// 3. Artifacts (contains the information obtained through git about the branches) - very few commands
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

    pub fn registry(&self) -> Rc<RegistryState> {
        match &self.registry {
            Some(r) => r.clone(),
            None => panic!("This command is configured to not require a registry"),
        }
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn create_canister_client(&self) -> anyhow::Result<CanisterClient> {
        let nns_url = self.network.get_nns_urls().first().expect("Should have at least one NNS url");

        match &self.ic_admin {
            Some(a) => match &a.neuron.auth {
                crate::auth::Auth::Hsm { pin, slot, key_id } => CanisterClient::from_hsm(pin.clone(), *slot, key_id.clone(), nns_url),
                crate::auth::Auth::Keyfile { path } => CanisterClient::from_key_file(path.clone(), nns_url),
            },
            None => CanisterClient::from_anonymous(nns_url),
        }
    }

    pub fn ic_admin(&self) -> Rc<IcAdminWrapper> {
        match &self.ic_admin {
            Some(a) => a.clone(),
            None => panic!("This command is configured to not require ic-admin"),
        }
    }

    pub fn runner(&self) -> Runner {
        let ic_admin = self.ic_admin();
        let registry = self.registry();

        Runner::new(ic_admin, registry)
    }
}
