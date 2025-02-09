use std::{cell::RefCell, rc::Rc, sync::Arc};

use ic_canisters::IcAgentCanisterClient;
use ic_management_backend::{
    health::HealthStatusQuerier,
    lazy_git::LazyGit,
    lazy_registry::LazyRegistry,
    proposal::{ProposalAgent, ProposalAgentImpl},
};
use ic_management_types::Network;
use log::warn;

use crate::{
    artifact_downloader::{ArtifactDownloader, ArtifactDownloaderImpl},
    auth::Neuron,
    commands::{Args, AuthOpts, AuthRequirement, ExecutableCommand, IcAdminVersion},
    cordoned_feature_fetcher::CordonedFeatureFetcher,
    ic_admin::{IcAdmin, IcAdminImpl},
    runner::Runner,
    store::Store,
    subnet_manager::SubnetManager,
};

#[derive(Clone)]
struct NeuronOpts {
    auth_opts: AuthOpts,
    requirement: AuthRequirement,
    neuron_id: Option<u64>,
    neuron_override: Option<Neuron>,
}

#[derive(Clone)]
pub struct DreContext {
    network: Network,
    registry: RefCell<Option<Arc<dyn LazyRegistry>>>,
    ic_admin: RefCell<Option<Arc<dyn IcAdmin>>>,
    runner: RefCell<Option<Rc<Runner>>>,
    ic_repo: RefCell<Option<Arc<dyn LazyGit>>>,
    proposal_agent: Arc<dyn ProposalAgent>,
    verbose_runner: bool,
    dry_run: bool,
    artifact_downloader: Arc<dyn ArtifactDownloader>,
    neuron: RefCell<Option<Neuron>>,
    proceed_without_confirmation: bool,
    version: IcAdminVersion,
    neuron_opts: NeuronOpts,
    cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
    health_client: Arc<dyn HealthStatusQuerier>,
    store: Store,
}

#[allow(clippy::too_many_arguments)]
impl DreContext {
    pub async fn new(
        network: Network,
        auth: AuthOpts,
        neuron_id: Option<u64>,
        verbose: bool,
        yes: bool,
        dry_run: bool,
        auth_requirement: AuthRequirement,
        ic_admin_version: IcAdminVersion,
        cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
        health_client: Arc<dyn HealthStatusQuerier>,
        store: Store,
        neuron_override: Option<Neuron>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            proposal_agent: Arc::new(ProposalAgentImpl::new(&network.nns_urls)),
            network,
            registry: RefCell::new(None),
            ic_admin: RefCell::new(None),
            runner: RefCell::new(None),
            verbose_runner: verbose,
            ic_repo: RefCell::new(None),
            dry_run,
            artifact_downloader: Arc::new(ArtifactDownloaderImpl {}) as Arc<dyn ArtifactDownloader>,
            neuron: RefCell::new(None),
            proceed_without_confirmation: yes,
            version: ic_admin_version,
            neuron_opts: NeuronOpts {
                auth_opts: auth,
                requirement: auth_requirement,
                neuron_id,
                neuron_override,
            },
            cordoned_features_fetcher,
            health_client,
            store,
        })
    }

    // Method that will be called from `main.rs` and
    // will return real implementations of services
    pub(crate) async fn from_args(args: &Args) -> anyhow::Result<Self> {
        let store = Store::new(args.offline)?;
        let network = match store.is_offline() {
            false => ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
                .await
                .map_err(|e| anyhow::anyhow!(e))?,
            true => Network::new_unchecked(args.network.clone(), &args.nns_urls)?,
        };
        Self::new(
            network.clone(),
            args.auth_opts.clone(),
            args.neuron_id,
            args.verbose,
            args.yes,
            args.dry_run,
            args.subcommands.require_auth(),
            args.ic_admin_version.clone(),
            store.cordoned_features_fetcher(args.cordoned_features_file.clone())?,
            store.health_client(&network)?,
            store,
            args.neuron_override(),
        )
        .await
    }

    pub async fn registry(&self) -> Arc<dyn LazyRegistry> {
        if let Some(reg) = self.registry.borrow().as_ref() {
            return reg.clone();
        }
        let registry = self.store.registry(self.network(), self.proposals_agent()).await.unwrap();
        *self.registry.borrow_mut() = Some(registry.clone());
        registry
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    #[must_use]
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn is_offline(&self) -> bool {
        self.store.is_offline()
    }

    /// Uses `ic_agent::Agent`
    pub async fn create_ic_agent_canister_client(&self) -> anyhow::Result<(Neuron, IcAgentCanisterClient)> {
        let neuron = self.neuron().await?;
        let canister_client = neuron.auth.clone().create_canister_client(self.network.get_nns_urls().to_vec())?;
        Ok((neuron, canister_client))
    }

    pub async fn ic_admin(&self) -> anyhow::Result<Arc<dyn IcAdmin>> {
        if let Some(a) = self.ic_admin.borrow().as_ref() {
            return Ok(a.clone());
        }

        let ic_admin = self
            .store
            .ic_admin(
                &self.version,
                self.network(),
                self.proceed_without_confirmation,
                self.neuron().await?,
                self.dry_run,
            )
            .await?;
        *self.ic_admin.borrow_mut() = Some(ic_admin.clone());
        Ok(ic_admin)
    }

    pub async fn neuron(&self) -> anyhow::Result<Neuron> {
        if let Some(n) = self.neuron.borrow().as_ref() {
            return Ok(n.clone());
        }

        let maybe_neuron = Neuron::from_opts_and_req(
            self.neuron_opts.auth_opts.clone(),
            self.neuron_opts.requirement.clone(),
            self.network(),
            self.neuron_opts.neuron_id,
            self.store.is_offline(),
            self.neuron_opts.neuron_override.clone(),
        )
        .await;

        // This code will add a fake neuron if it
        // cannot detect anything for the command
        let neuron = match maybe_neuron {
            Ok(n) => n,
            Err(e) => {
                warn!(
                    "Couldn't detect neuron due to: {:?}.  Will fall back to anonymous in dry-run operations.",
                    e
                );
                Neuron::dry_run_fake_neuron()?
            }
        };

        *self.neuron.borrow_mut() = Some(neuron.clone());
        Ok(neuron)
    }

    pub fn neuron_id(&self) -> Option<u64> {
        self.neuron_opts.neuron_id
    }

    pub async fn readonly_ic_admin_for_other_network(&self, network: Network) -> anyhow::Result<impl IcAdmin> {
        let ic_admin = self.ic_admin().await?;
        Ok(IcAdminImpl::new(
            network,
            ic_admin.ic_admin_path(),
            true,
            Neuron::anonymous_neuron(),
            false,
        ))
    }

    pub async fn subnet_manager(&self) -> anyhow::Result<SubnetManager> {
        let registry = self.registry().await;

        Ok(SubnetManager::new(
            registry,
            self.cordoned_features_fetcher.clone(),
            self.health_client.clone(),
        ))
    }

    pub fn proposals_agent(&self) -> Arc<dyn ProposalAgent> {
        self.proposal_agent.clone()
    }

    pub async fn runner(&self) -> anyhow::Result<Rc<Runner>> {
        if let Some(r) = self.runner.borrow().as_ref() {
            return Ok(r.clone());
        }

        let runner = Rc::new(Runner::new(
            self.registry().await,
            self.network().clone(),
            self.proposals_agent(),
            self.verbose_runner,
            self.ic_repo.clone(),
            self.artifact_downloader.clone(),
            self.cordoned_features_fetcher.clone(),
            self.health_client.clone(),
        ));
        *self.runner.borrow_mut() = Some(runner.clone());
        Ok(runner)
    }

    pub fn health_client(&self) -> Arc<dyn HealthStatusQuerier> {
        self.health_client.clone()
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub mod tests {
    use std::{cell::RefCell, sync::Arc};

    use ic_management_backend::{health::HealthStatusQuerier, lazy_git::LazyGit, lazy_registry::LazyRegistry, proposal::ProposalAgent};
    use ic_management_types::Network;

    use crate::{
        artifact_downloader::ArtifactDownloader,
        auth::Neuron,
        commands::{AuthOpts, /*DiscourseOpts,*/ HsmOpts, HsmParams},
        cordoned_feature_fetcher::CordonedFeatureFetcher,
        ic_admin::IcAdmin,
        store::Store,
    };

    use super::DreContext;

    pub fn get_mocked_ctx(
        network: Network,
        neuron: Neuron,
        registry: Arc<dyn LazyRegistry>,
        ic_admin: Arc<dyn IcAdmin>,
        git: Arc<dyn LazyGit>,
        proposal_agent: Arc<dyn ProposalAgent>,
        artifact_downloader: Arc<dyn ArtifactDownloader>,
        cordoned_features_fetcher: Arc<dyn CordonedFeatureFetcher>,
        health_client: Arc<dyn HealthStatusQuerier>,
    ) -> DreContext {
        DreContext {
            network,
            registry: RefCell::new(Some(registry)),
            ic_admin: RefCell::new(Some(ic_admin)),
            runner: RefCell::new(None),
            ic_repo: RefCell::new(Some(git)),
            proposal_agent,
            verbose_runner: true,
            dry_run: true,
            artifact_downloader,
            neuron: RefCell::new(Some(neuron.clone())),
            proceed_without_confirmation: true,
            version: crate::commands::IcAdminVersion::Strict("Shouldn't reach this because of mock".to_string()),
            neuron_opts: super::NeuronOpts {
                auth_opts: AuthOpts {
                    private_key_pem: None,
                    hsm_opts: HsmOpts {
                        hsm_pin: None,
                        hsm_params: HsmParams {
                            hsm_slot: None,
                            hsm_key_id: None,
                        },
                    },
                },
                neuron_override: None,
                requirement: crate::commands::AuthRequirement::Neuron,
                neuron_id: match neuron.neuron_id {
                    0 => None,
                    n => Some(n),
                },
            },
            cordoned_features_fetcher,
            health_client,
            store: Store::new(false).unwrap(),
        }
    }
}
