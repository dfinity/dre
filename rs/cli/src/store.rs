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

#[derive(serde::Deserialize)]
struct GitRefObject {
    sha: String,
}

#[derive(serde::Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    ref_name: String,
    object: GitRefObject,
}

// The earliest year for which dfinity/ic publishes dated `release-*` tags.
const EARLIEST_IC_RELEASE_YEAR: i32 = 2024;

/// Resolves an IC commit hash to its GitHub release tag (e.g.
/// `release-2026-06-04_04-52-base`).
///
/// IC release tags are named `release-YYYY-MM-DD_HH-MM-<variant>`, so they are
/// grouped per year via the `matching-refs` API. For the common case (version
/// derived from the registry canister or the embedded fallback) the commit is
/// always recent, so we only look at the current year's releases. A full scan
/// back to [`EARLIEST_IC_RELEASE_YEAR`] is only performed when the user pins an
/// arbitrary `--ic-admin-version <commit>`, which may point to an older release.
async fn find_github_release_tag_for_commit(commit: &str, deep_scan: bool) -> anyhow::Result<String> {
    use chrono::Datelike;
    let client = reqwest::Client::builder().user_agent("dre-cli").build()?;
    let current_year = chrono::Utc::now().year();

    // Always include the previous year as well, so lookups keep working in early
    // January before the first release of the new year is published.
    let oldest_year = if deep_scan {
        EARLIEST_IC_RELEASE_YEAR
    } else {
        (current_year - 1).max(EARLIEST_IC_RELEASE_YEAR)
    };

    // Authenticated requests get a much higher rate limit (5000/hour vs 60/hour).
    let github_token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());

    for year in (oldest_year..=current_year).rev() {
        let mut page = 1u32;
        loop {
            let url = format!("https://api.github.com/repos/dfinity/ic/git/matching-refs/tags/release-{}", year);
            let mut request = client.get(&url).query(&[("per_page", "100"), ("page", page.to_string().as_str())]);
            if let Some(token) = &github_token {
                request = request.bearer_auth(token);
            }
            let response = request.send().await?;

            // Surface GitHub rate-limiting explicitly, since unauthenticated requests
            // are capped at 60/hour per IP and the raw 403 is otherwise cryptic.
            if response.status() == reqwest::StatusCode::FORBIDDEN
                && response.headers().get("x-ratelimit-remaining").and_then(|v| v.to_str().ok()) == Some("0")
            {
                return Err(anyhow::anyhow!(
                    "GitHub API rate limit exceeded while resolving the ic-admin release for commit {}. \
                     Set the GITHUB_TOKEN environment variable to raise the limit, or pass an explicit \
                     binary with --ic-admin <path>.",
                    commit
                ));
            }
            let refs: Vec<GitRef> = response.error_for_status()?.json().await?;

            let count = refs.len();
            for git_ref in refs {
                if git_ref.object.sha == commit {
                    let tag = git_ref.ref_name.trim_start_matches("refs/tags/").to_string();
                    return Ok(tag);
                }
            }

            if count < 100 {
                break;
            }
            page += 1;
        }
    }

    let hint = if deep_scan {
        "The commit may not have an associated IC release. Pass an explicit binary with --ic-admin <path> if needed."
    } else {
        "Only recent releases were checked. If this is an older commit, pin it explicitly with --ic-admin-version <commit>."
    };
    Err(anyhow::anyhow!("No GitHub release tag found for commit {}. {}", commit, hint))
}

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
pub const FALLBACK_IC_ADMIN_VERSION: &str = "b95f4a32b41798de115aac9298b51dd1662f1da5";

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

    async fn download_ic_admin(&self, version: &str, path: &PathBuf, deep_scan: bool) -> anyhow::Result<()> {
        let tag = find_github_release_tag_for_commit(version, deep_scan).await?;
        let asset_name = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", _) => "ic-admin-arm64-darwin.gz",
            (_, "aarch64") => "ic-admin-arm64-linux.gz",
            _ => "ic-admin-x86_64-linux.gz",
        };
        let url = format!("https://github.com/dfinity/ic/releases/download/{tag}/{asset_name}");
        info!("Downloading ic-admin version: {} from {}", version, url);
        let body = reqwest::get(&url).await?.error_for_status()?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().ok_or(anyhow::anyhow!("Failed to get parent for ic admin revision dir"))?;
        fs_err::create_dir_all(path_parent).map_err(|_| anyhow::anyhow!("create_dir_all failed for {}", path_parent.display()))?;
        let mut out = fs_err::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        fs_err::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
        Ok(())
    }

    async fn init_ic_admin(&self, version: &str, network: &Network, neuron: Neuron, deep_scan: bool) -> anyhow::Result<Arc<IcAdminImpl>> {
        let path = self.ic_admin_path_for_version(version)?;

        if !path.exists() {
            self.download_ic_admin(version, &path, deep_scan).await?;
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
            // The fallback version is recent, so a shallow scan suffices.
            IcAdminVersion::Fallback => self.init_ic_admin(FALLBACK_IC_ADMIN_VERSION, network, neuron, false).await,
            // The user pinned an arbitrary commit which may be old; scan all releases.
            IcAdminVersion::Strict(ver) => self.init_ic_admin(ver, network, neuron, true).await,
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

                // The registry canister version tracks mainnet and is always recent.
                let ic_admin = self.init_ic_admin(&version, network, neuron, false).await?;

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
