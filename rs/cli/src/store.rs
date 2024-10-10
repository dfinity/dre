use std::{io::Read, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc, time::Duration};

use flate2::bufread::GzDecoder;
use ic_canisters::governance::governance_canister_version;
use ic_management_backend::{
    lazy_registry::{LazyRegistry, LazyRegistryImpl},
    proposal::ProposalAgent,
    registry::sync_local_store_with_path,
};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use log::{debug, info, warn};

use crate::{
    auth::Neuron,
    commands::IcAdminVersion,
    ic_admin::{IcAdmin, IcAdminImpl},
};

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    offline: bool,
}

const DURATION_BETWEEN_CHECKS_FOR_NEW_IC_ADMIN: Duration = Duration::from_secs(60 * 60 * 24);
pub const FALLBACK_IC_ADMIN_VERSION: &str = "d4ee25b0865e89d3eaac13a60f0016d5e3296b31";

impl Store {
    pub fn new(offline: bool) -> anyhow::Result<Self> {
        Ok(Self {
            path: dirs::cache_dir()
                .ok_or(anyhow::anyhow!("Couldn't find cache dir for dre store"))?
                .join("dre-store"),
            offline,
        })
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn local_store_for_network(&self, network: &Network) -> anyhow::Result<PathBuf> {
        let local_store_dir = self.path().join(&network.name).join("local_registry");

        if !local_store_dir.exists() {
            debug!(
                "Directory for local store for network {} doesn't exist. Creating on path `{}`",
                network.name,
                local_store_dir.display()
            );
            std::fs::create_dir_all(&local_store_dir)?
        }

        Ok(local_store_dir)
    }

    pub async fn registry(&self, network: &Network, proposal_agent: Arc<dyn ProposalAgent>) -> anyhow::Result<Arc<dyn LazyRegistry>> {
        let registry_path = self.local_store_for_network(network)?;

        info!("Using local registry path for network {}: {}", network.name, registry_path.display());

        match self.offline {
            true => warn!("Explicit offline mode! Registry won't be synced"),
            false => sync_local_store_with_path(network, &registry_path).await?,
        }

        let local_registry = LocalRegistry::new(&registry_path, Duration::from_millis(1000))?;

        Ok(Arc::new(LazyRegistryImpl::new(
            local_registry,
            network.clone(),
            self.offline,
            proposal_agent,
        )))
    }

    fn ic_admin_revision_dir(&self) -> anyhow::Result<PathBuf> {
        let path = self.path().join("ic-admin.revisions");

        if !path.exists() {
            info!("ic-admin.revisions dir was missing. Creating on path `{}`...", path.display());
            std::fs::create_dir_all(&path)?;
        }

        Ok(path)
    }

    fn ic_admin_status_file(&self) -> anyhow::Result<PathBuf> {
        let status_file = self.ic_admin_revision_dir()?.join("ic-admin.status");

        if !status_file.exists() {
            info!("ic-admin.status file was missing. Creating on path `{}`...", status_file.display());
            std::fs::write(&status_file, "")?
        }

        Ok(status_file)
    }

    fn ic_admin_path_for_version(&self, version: &str) -> anyhow::Result<PathBuf> {
        Ok(self.ic_admin_revision_dir()?.join(version).join("ic-admin"))
    }

    async fn download_ic_admin(&self, version: &str, path: &PathBuf) -> anyhow::Result<()> {
        let url = if std::env::consts::OS == "macos" {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-linux/ic-admin.gz")
        };
        info!("Downloading ic-admin version: {} from {}", version, url);
        let body = reqwest::get(url).await?.error_for_status()?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().ok_or(anyhow::anyhow!("Failed to get parent for ic admin revision dir"))?;
        std::fs::create_dir_all(path_parent).map_err(|_| anyhow::anyhow!("create_dir_all failed for {}", path_parent.display()))?;
        let mut out = std::fs::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
        Ok(())
    }

    async fn init_ic_admin(
        &self,
        version: &str,
        network: &Network,
        proceed_without_confirmation: bool,
        neuron: Neuron,
        dry_run: bool,
    ) -> anyhow::Result<Arc<dyn IcAdmin>> {
        let path = self.ic_admin_path_for_version(version)?;

        if !path.exists() {
            self.download_ic_admin(version, &path).await?;
        }

        info!("Using ic-admin: {}", path.display());
        Ok(Arc::new(IcAdminImpl::new(
            network.clone(),
            Some(
                path.to_str()
                    .ok_or(anyhow::anyhow!("Failed to convert ic-admin path to str"))?
                    .to_string(),
            ),
            proceed_without_confirmation,
            neuron,
            dry_run,
        )) as Arc<dyn IcAdmin>)
    }

    pub async fn ic_admin(
        &self,
        version: &IcAdminVersion,
        network: &Network,
        proceed_without_confirmation: bool,
        neuron: Neuron,
        dry_run: bool,
    ) -> anyhow::Result<Arc<dyn IcAdmin>> {
        match version {
            IcAdminVersion::Fallback => {
                return self
                    .init_ic_admin(FALLBACK_IC_ADMIN_VERSION, network, proceed_without_confirmation, neuron, dry_run)
                    .await
            }
            IcAdminVersion::Strict(ver) => return self.init_ic_admin(&ver, network, proceed_without_confirmation, neuron, dry_run).await,
            // This is the most probable way of running
            IcAdminVersion::FromGovernance => {
                let mut status_file = std::fs::File::open(&self.ic_admin_status_file()?)?;
                let elapsed = status_file.metadata()?.modified()?.elapsed().unwrap_or_default();

                let mut version_from_file = "".to_string();
                status_file.read_to_string(&mut version_from_file)?;

                let version = match (self.offline, version_from_file) {
                    // Running offline mode, no ic-admin present.
                    (true, version_from_file) if version_from_file.is_empty() => {
                        return Err(anyhow::anyhow!("No ic-admin version found and offline mode is specified"))
                    }
                    // Running offline mode and ic-admin version is present.
                    (true, version_from_file) => {
                        warn!("Offline mode specified! Will use cached ic-admin version: {}", version_from_file);
                        version_from_file
                    }
                    // Running online mode
                    //
                    // There is a cached version of ic-admin
                    // and the cached version is still younger
                    // than `DURATION_BETWEEN_CHECKS_FOR_NEW_IC_ADMIN`
                    (false, version_from_file) if !version_from_file.is_empty() && elapsed <= DURATION_BETWEEN_CHECKS_FOR_NEW_IC_ADMIN => {
                        info!("Using cached ic admin version: {}", version_from_file);
                        version_from_file
                    }
                    // Either there isn't a cached version at all
                    // or the `DURATION_BETWEEN_CHECKS_FOR_NEW_IC_ADMIN`
                    // has passed which means that the cache is invalid.
                    // Check should be performed
                    (false, _) => {
                        info!("Checking for new ic-admin version");
                        let govn_canister_version = governance_canister_version(&network.get_nns_urls()).await?;
                        debug!(
                            "Using ic-admin matching the version of governance canister, version: {}",
                            govn_canister_version.stringified_hash
                        );
                        let version = match govn_canister_version.stringified_hash.as_str() {
                            // This usually happens on testnets deployed
                            // from the HEAD of branch
                            "0000000000000000000000000000000000000000" => FALLBACK_IC_ADMIN_VERSION,
                            v => v,
                        };
                        version.to_string()
                    }
                };

                let ic_admin = self
                    .init_ic_admin(&version, network, proceed_without_confirmation, neuron, dry_run)
                    .await?;

                // Only update file when the sync
                // with governance has been performed
                std::fs::write(self.ic_admin_status_file()?, version)?;
                Ok(ic_admin)
            }
        }
    }

    pub fn cordoned_features_file(&self) -> anyhow::Result<PathBuf> {
        let file = self.path().join("cordoned_features.yaml");

        if !file.exists() {
            info!("Cordoned features file was missing. Creating on path `{}`...", file.display());
            if let Err(e) = std::fs::write(&file, "") {
                warn!("Failed to create cordoned features file: {:?}", e);
                warn!("This is not critical now. If github is offline then it could be");
            }
        }

        Ok(file)
    }
}
