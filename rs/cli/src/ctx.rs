use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use ic_canisters::{governance::governance_canister_version, IcAgentCanisterClient};
use ic_management_backend::{
    lazy_git::LazyGit,
    lazy_registry::{LazyRegistry, LazyRegistryImpl},
    proposal::{ProposalAgent, ProposalAgentImpl},
    registry::{local_registry_path, sync_local_store},
};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use log::{debug, info, warn};
use url::Url;

use crate::{
    artifact_downloader::{ArtifactDownloader, ArtifactDownloaderImpl},
    auth::Neuron,
    commands::{Args, AuthOpts, AuthRequirement, ExecutableCommand, IcAdminVersion},
    ic_admin::{download_ic_admin, should_update_ic_admin, IcAdmin, IcAdminImpl, FALLBACK_IC_ADMIN_VERSION},
    runner::Runner,
    subnet_manager::SubnetManager,
};

#[derive(Clone)]
pub struct DreContext {
    network: Network,
    registry: RefCell<Option<Arc<dyn LazyRegistry>>>,
    ic_admin: RefCell<Option<Arc<dyn IcAdmin>>>,
    runner: RefCell<Option<Rc<Runner>>>,
    ic_repo: RefCell<Option<Arc<dyn LazyGit>>>,
    proposal_agent: Arc<dyn ProposalAgent>,
    verbose_runner: bool,
    skip_sync: bool,
    forum_post_link: Option<String>,
    dry_run: bool,
    artifact_downloader: Arc<dyn ArtifactDownloader>,
    neuron: Neuron,
    proceed_without_confirmation: bool,
    version: IcAdminVersion,
}

impl DreContext {
    pub async fn new(
        network: String,
        nns_urls: Vec<Url>,
        auth: AuthOpts,
        neuron_id: Option<u64>,
        verbose: bool,
        no_sync: bool,
        yes: bool,
        dry_run: bool,
        auth_requirement: AuthRequirement,
        forum_post_link: Option<String>,
        ic_admin_version: IcAdminVersion,
    ) -> anyhow::Result<Self> {
        let network = match no_sync {
            false => ic_management_types::Network::new(network.clone(), &nns_urls)
                .await
                .map_err(|e| anyhow::anyhow!(e))?,
            true => Network::new_unchecked(network.clone(), &nns_urls)?,
        };

        let maybe_neuron = Neuron::from_opts_and_req(auth, auth_requirement, &network, neuron_id).await;
        let neuron = match dry_run {
            true => match maybe_neuron {
                Ok(n) => n,
                Err(e) => {
                    warn!("Couldn't detect neuron due to: {:?}", e);
                    warn!("Falling back to Annonymous for dry-run");
                    Neuron::dry_run_fake_neuron(&network).await?
                }
            },
            false => maybe_neuron?,
        };

        Ok(Self {
            proposal_agent: Arc::new(ProposalAgentImpl::new(&network.nns_urls)),
            network,
            registry: RefCell::new(None),
            ic_admin: RefCell::new(None),
            runner: RefCell::new(None),
            verbose_runner: verbose,
            skip_sync: no_sync,
            forum_post_link: forum_post_link.clone(),
            ic_repo: RefCell::new(None),
            dry_run,
            artifact_downloader: Arc::new(ArtifactDownloaderImpl {}) as Arc<dyn ArtifactDownloader>,
            neuron,
            proceed_without_confirmation: yes,
            version: ic_admin_version,
        })
    }

    pub(crate) async fn from_args(args: &Args) -> anyhow::Result<Self> {
        Self::new(
            args.network.clone(),
            args.nns_urls.clone(),
            args.auth_opts.clone(),
            args.neuron_id,
            args.verbose,
            args.no_sync,
            args.yes,
            args.dry_run,
            args.subcommands.require_auth(),
            args.forum_post_link.clone(),
            args.ic_admin_version.clone(),
        )
        .await
    }

    pub async fn registry(&self) -> Arc<dyn LazyRegistry> {
        if let Some(reg) = self.registry.borrow().as_ref() {
            return reg.clone();
        }
        let network = self.network();

        if !self.skip_sync {
            sync_local_store(network).await.expect("Should be able to sync registry");
        }
        let local_path = local_registry_path(network);
        info!("Using local registry path for network {}: {}", network.name, local_path.display());
        let local_registry = LocalRegistry::new(local_path, Duration::from_millis(1000)).expect("Failed to create local registry");

        let registry = Arc::new(LazyRegistryImpl::new(
            local_registry,
            network.clone(),
            self.skip_sync,
            self.proposals_agent(),
        ));
        *self.registry.borrow_mut() = Some(registry.clone());
        registry
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Uses `ic_agent::Agent`
    pub fn create_ic_agent_canister_client(&self, lock: Option<Mutex<()>>) -> anyhow::Result<IcAgentCanisterClient> {
        self.neuron.auth.create_canister_client(self.network.get_nns_urls().to_vec(), lock)
    }

    pub async fn ic_admin(&self) -> anyhow::Result<Arc<dyn IcAdmin>> {
        if let Some(a) = self.ic_admin.borrow().as_ref() {
            return Ok(a.clone());
        }

        let ic_admin_path = match &self.version {
            IcAdminVersion::FromGovernance => match should_update_ic_admin()? {
                (true, _) => {
                    let govn_canister_version = governance_canister_version(self.network().get_nns_urls()).await?;
                    debug!(
                        "Using ic-admin matching the version of governance canister, version: {}",
                        govn_canister_version.stringified_hash
                    );
                    download_ic_admin(match govn_canister_version.stringified_hash.as_str() {
                        // Some testnets could have this version setup if deployed
                        // from HEAD of the branch they are created from
                        "0000000000000000000000000000000000000000" => None,
                        v => Some(v.to_owned()),
                    })
                    .await?
                }
                (false, s) => {
                    debug!("Using cached ic-admin matching the version of governance canister, path: {}", s);
                    s
                }
            },
            IcAdminVersion::Fallback => {
                debug!("Using default ic-admin, version: {}", FALLBACK_IC_ADMIN_VERSION);
                download_ic_admin(None).await?
            }
            IcAdminVersion::Strict(ver) => {
                debug!("Using ic-admin specified via args: {}", ver);
                download_ic_admin(Some(ver.to_string())).await?
            }
        };

        let ic_admin = Arc::new(IcAdminImpl::new(
            self.network().clone(),
            Some(ic_admin_path.clone()),
            self.proceed_without_confirmation,
            self.neuron(),
            self.dry_run,
        )) as Arc<dyn IcAdmin>;

        *self.ic_admin.borrow_mut() = Some(ic_admin.clone());
        Ok(ic_admin)
    }

    pub fn neuron(&self) -> Neuron {
        self.neuron.clone()
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

    pub async fn subnet_manager(&self) -> SubnetManager {
        let registry = self.registry().await;

        SubnetManager::new(registry, self.network().clone())
    }

    pub fn proposals_agent(&self) -> Arc<dyn ProposalAgent> {
        self.proposal_agent.clone()
    }

    pub async fn runner(&self) -> anyhow::Result<Rc<Runner>> {
        if let Some(r) = self.runner.borrow().as_ref() {
            return Ok(r.clone());
        }

        let runner = Rc::new(Runner::new(
            self.ic_admin().await?,
            self.registry().await,
            self.network().clone(),
            self.proposals_agent(),
            self.verbose_runner,
            self.ic_repo.clone(),
            self.artifact_downloader.clone(),
        ));
        *self.runner.borrow_mut() = Some(runner.clone());
        Ok(runner)
    }

    pub fn forum_post_link(&self) -> Option<String> {
        self.forum_post_link.clone()
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub mod tests {
    use std::{cell::RefCell, sync::Arc};

    use ic_management_backend::{lazy_git::LazyGit, lazy_registry::LazyRegistry, proposal::ProposalAgent};
    use ic_management_types::Network;

    use crate::{artifact_downloader::ArtifactDownloader, auth::Neuron, ic_admin::IcAdmin};

    use super::DreContext;

    pub fn get_mocked_ctx(
        network: Network,
        neuron: Neuron,
        registry: Arc<dyn LazyRegistry>,
        ic_admin: Arc<dyn IcAdmin>,
        git: Arc<dyn LazyGit>,
        proposal_agent: Arc<dyn ProposalAgent>,
        artifact_downloader: Arc<dyn ArtifactDownloader>,
    ) -> DreContext {
        DreContext {
            network,
            registry: RefCell::new(Some(registry)),
            ic_admin: RefCell::new(Some(ic_admin)),
            runner: RefCell::new(None),
            ic_repo: RefCell::new(Some(git)),
            proposal_agent,
            verbose_runner: true,
            skip_sync: false,
            forum_post_link: None,
            dry_run: true,
            artifact_downloader,
            neuron,
            proceed_without_confirmation: true,
            version: crate::commands::IcAdminVersion::Strict("Shouldn't reach this because of mock".to_string()),
        }
    }
}
