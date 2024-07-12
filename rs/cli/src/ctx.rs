use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use ic_canisters::{governance::governance_canister_version, CanisterClient, IcAgentCanisterClient};
use ic_management_backend::{
    lazy_registry::LazyRegistry,
    proposal::ProposalAgent,
    registry::{local_registry_path, sync_local_store},
};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use log::info;

use crate::{
    auth::Neuron,
    commands::{Args, ExecutableCommand, IcAdminRequirement},
    ic_admin::{download_ic_admin, should_update_ic_admin, IcAdminWrapper},
    runner::Runner,
    subnet_manager::SubnetManager,
};

const STAGING_NEURON_ID: u64 = 49;
pub struct DreContext {
    network: Network,
    registry: RefCell<Option<Rc<LazyRegistry>>>,
    ic_admin: Option<Arc<IcAdminWrapper>>,
    runner: RefCell<Option<Rc<Runner>>>,
    verbose_runner: bool,
}

impl DreContext {
    pub async fn from_args(args: &Args) -> anyhow::Result<Self> {
        let network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let (neuron_id, private_key_pem) = {
            let neuron_id = match args.neuron_id {
                Some(n) => Some(n),
                None if network.name == "staging" => Some(STAGING_NEURON_ID),
                None => None,
            };

            let path = PathBuf::from_str(&std::env::var("HOME")?)?.join(".config/dfx/identity/bootstrap-super-leader/identity.pem");
            let private_key_pem = match args.private_key_pem.as_ref() {
                Some(p) => Some(p.clone()),
                None if network.name == "staging" && path.exists() => Some(path),
                None => None,
            };
            (neuron_id, private_key_pem)
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
            runner: RefCell::new(None),
            verbose_runner: args.verbose,
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
    ) -> anyhow::Result<Option<Arc<IcAdminWrapper>>> {
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
                Neuron::new(private_key_pem, hsm_slot, hsm_pin.clone(), hsm_key_id.clone(), neuron_id, network, true).await?
            }
            IcAdminRequirement::OverridableBy {
                network: accepted_network,
                neuron,
            } => {
                let maybe_neuron = Neuron::new(private_key_pem, hsm_slot, hsm_pin.clone(), hsm_key_id.clone(), neuron_id, network, true).await;

                match maybe_neuron {
                    Ok(n) => n,
                    Err(_) if accepted_network == *network => neuron,
                    Err(e) => return Err(e),
                }
            }
        };
        let ic_admin_path = match should_update_ic_admin()? {
            (true, _) => {
                let govn_canister_version = governance_canister_version(network.get_nns_urls()).await?;
                download_ic_admin(Some(govn_canister_version.stringified_hash)).await?
            }
            (false, s) => s,
        };

        let ic_admin = Some(Arc::new(IcAdminWrapper::new(
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

    /// Uses `ic_canister_client::Agent`
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

    /// Uses `ic_agent::Agent`
    pub fn create_ic_agent_canister_client(&self, lock: Option<Mutex<()>>) -> anyhow::Result<IcAgentCanisterClient> {
        let nns_url = self.network.get_nns_urls().first().expect("Should have at least one NNS url");
        match &self.ic_admin {
            Some(a) => match &a.neuron.auth {
                crate::auth::Auth::Hsm { pin, slot, key_id } => {
                    IcAgentCanisterClient::from_hsm(pin.to_string(), *slot, key_id.to_string(), nns_url.to_owned(), lock)
                }
                crate::auth::Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.into(), nns_url.to_owned()),
                crate::auth::Auth::Anonymous => IcAgentCanisterClient::from_anonymous(nns_url.to_owned()),
            },
            None => IcAgentCanisterClient::from_anonymous(nns_url.to_owned()),
        }
    }

    pub fn ic_admin(&self) -> Arc<IcAdminWrapper> {
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

    pub async fn runner(&self) -> Rc<Runner> {
        if let Some(r) = self.runner.borrow().as_ref() {
            return r.clone();
        }

        let runner = Rc::new(Runner::new(
            self.ic_admin(),
            self.registry().await,
            self.network().clone(),
            self.proposals_agent(),
            self.verbose_runner,
        ));
        *self.runner.borrow_mut() = Some(runner.clone());
        runner
    }
}
