use flate2::bufread::GzDecoder;
use ic_canisters::registry::registry_canister_version;
use ic_management_backend::{
    health::{HealthClient, HealthStatusQuerier},
    lazy_registry::{LazyRegistry, LazyRegistryImpl},
    proposal::ProposalAgent,
    registry::sync_local_store_with_path,
};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use log::{debug, info, warn};
use std::os::unix::fs::PermissionsExt;
use std::{io::Read, path::PathBuf, sync::Arc, time::Duration};

use crate::{
    auth::Neuron,
    cordoned_feature_fetcher::{CordonedFeatureFetcher, CordonedFeatureFetcherImpl},
    exe::args::IcAdminVersion,
    ic_admin::IcAdminImpl,
};

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    offline: bool,
}

const DURATION_BETWEEN_CHECKS_FOR_NEW_IC_ADMIN: Duration = Duration::from_secs(60 * 60 * 24);
pub const FALLBACK_IC_ADMIN_VERSION: &str = "1a1cb8cbff5e5c5c1fd01ec37e3c22e5119f12c3";

impl Store {
    #[cfg(not(test))]
    pub fn new(offline: bool) -> anyhow::Result<Self> {
        Self::new_inner(
            offline,
            dirs::cache_dir()
                .ok_or(anyhow::anyhow!("Couldn't find cache dir for dre store"))?
                .join("dre-store"),
        )
    }

    // Really important to distinguish from test and
    // real store because test store, if not handled
    // correctly, can leave an invalid state
    #[cfg(test)]
    pub fn new(offline: bool) -> anyhow::Result<Self> {
        use std::str::FromStr;

        Self::new_inner(offline, PathBuf::from_str("/tmp").unwrap().join("dre-test-store"))
    }

    fn new_inner(offline: bool, path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            fs_err::create_dir_all(&path)?;
        }
        Ok(Self { path, offline })
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn local_store_for_network(&self, network: &Network) -> anyhow::Result<PathBuf> {
        let local_store_dir = self.path().join("local_registry").join(&network.name);

        if !local_store_dir.exists() {
            debug!(
                "Directory for local store for network {} doesn't exist. Creating on path `{}`",
                network.name,
                local_store_dir.display()
            );
            fs_err::create_dir_all(&local_store_dir)?
        }

        Ok(local_store_dir)
    }

    fn guest_labels_cache_dir(&self, network: &Network) -> anyhow::Result<PathBuf> {
        let dir = self.path().join("labels").join(&network.name);

        if !dir.exists() {
            debug!(
                "Directory for labels for network {} doesn't exist. Creating on path `{}`",
                network.name,
                dir.display()
            );
            fs_err::create_dir_all(&dir)?
        }

        Ok(dir)
    }

    #[cfg(test)]
    pub fn guest_labels_cache_path_outer(&self, network: &Network) -> anyhow::Result<PathBuf> {
        self.guest_labels_cache_path(network)
    }

    fn guest_labels_cache_path(&self, network: &Network) -> anyhow::Result<PathBuf> {
        let path = self.guest_labels_cache_dir(network)?.join("labels.yaml");

        if !path.exists() {
            fs_err::write(&path, "")?;
        }

        Ok(path)
    }

    pub async fn registry(
        &self,
        network: &Network,
        proposal_agent: Arc<dyn ProposalAgent>,
        version_height: Option<u64>,
    ) -> anyhow::Result<Arc<dyn LazyRegistry>> {
        let registry_path = self.local_store_for_network(network)?;

        info!(
            "Using local registry path for network {}: {} (height: {}, offline: {})",
            network.name,
            registry_path.display(),
            version_height.map(|v| v.to_string()).unwrap_or_else(|| "latest".to_string()),
            self.offline,
        );

        match self.offline {
            true => {
                if self.offline {
                    warn!("Explicit offline mode! Registry won't be synced")
                }
            }
            false => sync_local_store_with_path(network, &registry_path).await?,
        }

        let local_registry = LocalRegistry::new(&registry_path, Duration::from_millis(1000))?;

        Ok(Arc::new(LazyRegistryImpl::new(
            local_registry,
            network.clone(),
            self.offline,
            proposal_agent,
            self.guest_labels_cache_path(network)?,
            self.health_client(network)?,
            version_height,
        )))
    }

    fn ic_admin_revision_dir(&self) -> anyhow::Result<PathBuf> {
        let path = self.path().join("ic-admin.revisions");

        if !path.exists() {
            info!("ic-admin.revisions dir was missing. Creating on path `{}`...", path.display());
            fs_err::create_dir_all(&path)?;
        }

        Ok(path)
    }

    #[cfg(test)]
    pub fn ic_admin_status_file_outer(&self) -> anyhow::Result<PathBuf> {
        self.ic_admin_status_file()
    }

    fn ic_admin_status_file(&self) -> anyhow::Result<PathBuf> {
        let status_file = self.ic_admin_revision_dir()?.join("ic-admin.status");

        if !status_file.exists() {
            info!("ic-admin.status file was missing. Creating on path `{}`...", status_file.display());
            fs_err::write(&status_file, "")?
        }

        Ok(status_file)
    }

    fn ic_admin_path_for_version(&self, version: &str) -> anyhow::Result<PathBuf> {
        Ok(self.ic_admin_revision_dir()?.join(version).join("ic-admin"))
    }

    async fn download_ic_admin(&self, version: &str, path: &PathBuf) -> anyhow::Result<()> {
        let url = if std::env::consts::OS == "macos" {
            // Apple Silicon will emulate x86 architecture.
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-linux/ic-admin.gz")
        };
        info!("Downloading ic-admin version: {} from {}", version, url);
        let body = reqwest::get(url).await?.error_for_status()?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().ok_or(anyhow::anyhow!("Failed to get parent for ic admin revision dir"))?;
        fs_err::create_dir_all(path_parent).map_err(|_| anyhow::anyhow!("create_dir_all failed for {}", path_parent.display()))?;
        let mut out = fs_err::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        fs_err::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
        Ok(())
    }

    async fn init_ic_admin(&self, version: &str, network: &Network, neuron: Neuron) -> anyhow::Result<Arc<IcAdminImpl>> {
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
            neuron,
        )))
    }

    pub async fn ic_admin(&self, version: &IcAdminVersion, network: &Network, neuron: Neuron) -> anyhow::Result<Arc<IcAdminImpl>> {
        match version {
            IcAdminVersion::Fallback => self.init_ic_admin(FALLBACK_IC_ADMIN_VERSION, network, neuron).await,
            IcAdminVersion::Strict(ver) => self.init_ic_admin(ver, network, neuron).await,
            // This is the most probable way of running
            IcAdminVersion::FromRegistry => {
                let mut status_file = fs_err::File::open(&self.ic_admin_status_file()?)?;
                let elapsed = status_file.metadata()?.modified()?.elapsed().unwrap_or_default();

                let mut version_from_file = "".to_string();
                status_file.read_to_string(&mut version_from_file)?;

                let version = match (self.offline, version_from_file) {
                    // Running offline mode, no ic-admin present.
                    (true, version_from_file) if version_from_file.is_empty() => {
                        return Err(anyhow::anyhow!("No ic-admin version found and offline mode is specified"));
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
                        let registry_version = registry_canister_version(network.get_nns_urls()[0].clone()).await?;
                        debug!(
                            "Using ic-admin matching the version of registry canister, version: {}",
                            registry_version.stringified_hash
                        );
                        let version = match registry_version.stringified_hash.as_str() {
                            // This usually happens on testnets deployed
                            // from the HEAD of branch
                            "0000000000000000000000000000000000000000" => FALLBACK_IC_ADMIN_VERSION,
                            v => v,
                        };
                        version.to_string()
                    }
                };

                let ic_admin = self.init_ic_admin(&version, network, neuron).await?;

                // Only update file when the sync
                // with governance has been performed
                fs_err::write(self.ic_admin_status_file()?, version)?;
                Ok(ic_admin)
            }
        }
    }

    #[cfg(test)]
    pub fn cordoned_features_file_outer(&self) -> anyhow::Result<PathBuf> {
        self.cordoned_features_file(None)
    }

    fn cordoned_features_file(&self, file_path: Option<String>) -> anyhow::Result<PathBuf> {
        let file = match file_path {
            Some(path) => std::path::PathBuf::from(path).canonicalize()?,
            None => {
                let file = self.path().join("cordoned_features.yaml");

                if !file.exists() {
                    info!("Cordoned features file was missing. Creating on path `{}`...", file.display());
                    fs_err::write(&file, "")?;
                }

                file
            }
        };

        Ok(file)
    }

    pub fn cordoned_features_fetcher(&self, local_file_path: Option<String>) -> anyhow::Result<Arc<dyn CordonedFeatureFetcher>> {
        let file = self.cordoned_features_file(local_file_path.clone())?;
        Ok(Arc::new(CordonedFeatureFetcherImpl::new(
            file,
            self.is_offline() || local_file_path.is_some(),
        )?))
    }

    #[cfg(test)]
    pub fn node_health_file_outer(&self, network: &Network) -> anyhow::Result<PathBuf> {
        self.node_health_file(network)
    }

    fn node_health_file(&self, network: &Network) -> anyhow::Result<PathBuf> {
        let file = self.path().join("node_healths").join(&network.name).join("node_healths.json");

        if !file.exists() {
            info!("Node health file was missing. Creating on path `{}`...", file.display());
            fs_err::create_dir_all(file.parent().unwrap())?;
            fs_err::write(&file, "")?;
        }

        Ok(file)
    }

    pub fn health_client(&self, network: &Network) -> anyhow::Result<Arc<dyn HealthStatusQuerier>> {
        let file = self.node_health_file(network)?;

        Ok(Arc::new(HealthClient::new(network.clone(), Some(file), self.is_offline())))
    }
}
