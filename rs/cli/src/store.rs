use std::{path::PathBuf, sync::Arc, time::Duration};

use ic_management_backend::{
    lazy_registry::{LazyRegistry, LazyRegistryImpl},
    proposal::ProposalAgent,
    registry::sync_local_store_with_path,
};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;
use log::{debug, info, warn};

use crate::{commands::IcAdminVersion, ic_admin::IcAdmin};

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

    pub async fn ic_admin(&self, version: IcAdminVersion) -> anyhow::Result<Arc<dyn IcAdmin>> {
        let mut status_file = std::fs::File::open(&self.ic_admin_status_file()?)?;
        let elapsed = status_file.metadata()?.modified()?.elapsed().unwrap_or_default();

        let version_from_file = match self.offline {
            true => {}  // Check if there is any version in file
            false => {} // Check if the elapsed time passed and then determine if you should update
        };

        Ok(())
    }
}
