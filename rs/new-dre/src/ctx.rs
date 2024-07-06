use std::{path::PathBuf, rc::Rc, str::FromStr, time::Duration};

use ic_canisters::{governance::governance_canister_version, CanisterClient};
use ic_management_backend::{
    git_ic_repo::IcRepo,
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, local_registry_path, sync_local_store, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};
use ic_registry_local_registry::LocalRegistry;

use crate::{
    auth::Neuron,
    commands::{Args, ExecutableCommand, RegistryRequirement},
    ic_admin::{download_ic_admin, IcAdminWrapper},
    runner::Runner,
};

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    registry: Option<Rc<Registry>>,
    ic_admin: Option<Rc<IcAdminWrapper>>,
}

pub enum Registry {
    Synced(LocalRegistry),
    WithNodeDetails(Rc<RegistryState>),
    WithGitInfo(Rc<RegistryState>),
}

impl Registry {
    pub fn as_synced(&self) -> &LocalRegistry {
        match &self {
            Registry::Synced(r) => r,
            _ => panic!("This registry is configured to be of type Synced"),
        }
    }
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

        let registry = Self::init_registry(&network, args.require_registry()).await?;

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
    async fn init_registry(network: &Network, requirement: RegistryRequirement) -> anyhow::Result<Option<Rc<Registry>>> {
        let mut registry = match requirement {
            RegistryRequirement::None => return Ok(None),
            RegistryRequirement::Synced => {
                sync_local_store(&network).await?;
                let local_registry_path = local_registry_path(&network.clone());
                return Ok(Some(Rc::new(Registry::Synced(LocalRegistry::new(
                    local_registry_path,
                    Duration::from_millis(1000),
                )?))));
            }
            RegistryRequirement::WithNodeDetails => RegistryState::new(network, true, None).await,
            RegistryRequirement::WithGitInfo => {
                RegistryState::new(network, true, Some(IcRepo::new().expect("Should be able to create IC repo"))).await
            }
        };

        // Fetch node providers
        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(network, "v3/node-providers")
            .await?
            .node_providers;

        fetch_and_add_node_labels_guests_to_registry(network, &mut registry).await;

        // Update node details
        registry.update_only_node_details(&node_providers).await?;

        if let RegistryRequirement::WithGitInfo = requirement {
            registry.update_releases().await?;
        }

        let registry = Rc::new(registry);
        Ok(Some(Rc::new(match requirement {
            RegistryRequirement::None | RegistryRequirement::Synced => unreachable!("Shouldn't happen"),
            RegistryRequirement::WithNodeDetails => Registry::WithNodeDetails(registry),
            RegistryRequirement::WithGitInfo => Registry::WithGitInfo(registry),
        })))
    }

    pub fn registry(&self) -> Rc<Registry> {
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

        Runner::new(
            ic_admin,
            match registry.as_ref() {
                Registry::Synced(_) => unreachable!("Shouldn't happen"),
                Registry::WithNodeDetails(r) => r.to_owned(),
                Registry::WithGitInfo(r) => r.to_owned(),
            },
        )
    }
}
