use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr, time::Duration};

use ic_canisters::{governance::governance_canister_version, CanisterClient};
use ic_management_backend::{
    git_ic_repo::IcRepo,
    lazy_registry::LazyRegistry,
    proposal::ProposalAgent,
    public_dashboard::query_ic_dashboard_list,
    registry::{fetch_and_add_node_labels_guests_to_registry, local_registry_path, sync_local_store, RegistryState},
};
use ic_management_types::{Network, NodeProvidersResponse};
use ic_registry_local_registry::LocalRegistry;
use log::info;

use crate::{
    auth::Neuron,
    commands::{Args, ExecutableCommand, IcAdminRequirement},
    ic_admin::{download_ic_admin, should_update_ic_admin, IcAdminWrapper},
    subnet_manager::SubnetManager,
};

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    registry: RefCell<Option<Rc<LazyRegistry>>>,
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

        let ic_admin = Self::init_ic_admin(
            &network,
            neuron_id,
            private_key_pem,
            args.hsm_slot,
            args.hsm_key_id.clone(),
            args.hsm_pin.clone(),
            args.yes,
            args.dry_run,
            args.require_ic_admin(),
        )
        .await?;

        Ok(Self {
            network,
            registry: RefCell::new(None),
            ic_admin,
        })
    }

    async fn init_ic_admin(
        network: &Network,
        neuron_id: Option<u64>,
        private_key_pem: Option<PathBuf>,
        hsm_slot: Option<u64>,
        hsm_key_id: Option<String>,
        hsm_pin: Option<String>,
        proceed_without_confirmation: bool,
        dry_run: bool,
        requirement: IcAdminRequirement,
    ) -> anyhow::Result<Option<Rc<IcAdminWrapper>>> {
        if let IcAdminRequirement::None = requirement {
            return Ok(None);
        }

        let neuron = match requirement {
            IcAdminRequirement::Anonymous | IcAdminRequirement::None => Neuron {
                auth: crate::auth::Auth::Anonymous,
                neuron_id: 0,
                include_proposer: false,
            },
            IcAdminRequirement::Detect => {
                Neuron::new(private_key_pem, hsm_slot, hsm_pin.clone(), hsm_key_id.clone(), neuron_id, &network, true).await?
            }
            IcAdminRequirement::OverridableBy {
                network: accepted_network,
                neuron,
            } => {
                let maybe_neuron = Neuron::new(private_key_pem, hsm_slot, hsm_pin.clone(), hsm_key_id.clone(), neuron_id, &network, true).await;

                let neuron = match maybe_neuron {
                    Ok(n) => n,
                    Err(_) if accepted_network == *network => neuron,
                    Err(e) => return Err(e),
                };

                neuron
            }
        };

        let ic_admin_path = match should_update_ic_admin()? {
            (true, _) => {
                let govn_canister_version = governance_canister_version(network.get_nns_urls()).await?;
                download_ic_admin(Some(govn_canister_version.stringified_hash)).await?
            }
            (false, s) => s,
        };

        let ic_admin = Some(Rc::new(IcAdminWrapper::new(
            network.clone(),
            Some(ic_admin_path),
            proceed_without_confirmation,
            neuron,
            dry_run,
        )));

        Ok(ic_admin)
    }

    pub async fn registry(&self) -> Rc<LazyRegistry> {
        if let Some(reg) = self.registry.borrow().as_ref() {
            return reg.clone();
        }
        let network = self.network();

        sync_local_store(network).await.expect("Should be able to sync registry");
        let local_path = local_registry_path(network);
        info!("Using local registry path for network {}: {}", network.name, local_path.display());
        let local_registry = LocalRegistry::new(local_path, Duration::from_millis(1000)).expect("Failed to create local registry");

        let registry = Rc::new(LazyRegistry::new(local_registry, network.clone()));
        *self.registry.borrow_mut() = Some(registry.clone());
        registry
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
                crate::auth::Auth::Anonymous => CanisterClient::from_anonymous(nns_url),
            },
            None => CanisterClient::from_anonymous(nns_url),
        }
    }

    pub fn ic_admin(&self) -> Rc<IcAdminWrapper> {
        match &self.ic_admin {
            Some(a) => a.clone(),
            None => panic!("This command is not configured to use ic admin"),
        }
    }

    pub async fn subnet_manager(&self) -> SubnetManager {
        let registry = self.registry().await;

        SubnetManager::new(registry, self.network().clone())
    }

    pub fn proposals_agent(&self) -> ProposalAgent {
        ProposalAgent::new(self.network().get_nns_urls())
    }
}
