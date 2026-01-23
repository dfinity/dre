//! Application state management

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use slog::{Logger, error, info, warn};
use tokio::sync::watch;
use url::Url;

use crate::config::AppConfig;
use crate::gcp::GcpClient;
use crate::models::SubnetProposal;
use crate::registry::RegistryManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Logger
    pub log: Logger,
    /// Application configuration (GCP project, zones, etc.)
    pub config: Arc<AppConfig>,
    /// GCP client
    pub gcp_client: Arc<GcpClient>,
    /// Registry manager for ICP node data
    pub registry_manager: Arc<RegistryManager>,
    /// Subnet proposals (proposal_id -> SubnetProposal)
    pub subnet_proposals: Arc<DashMap<String, SubnetProposal>>,
    /// Poll interval for registry sync
    poll_interval: Duration,
}

impl AppState {
    /// Create a new application state
    pub async fn new(
        log: Logger,
        targets_dir: PathBuf,
        nns_url: Url,
        poll_interval: Duration,
        registry_query_timeout: Duration,
        gcp_credentials_file: Option<PathBuf>,
        config: AppConfig,
    ) -> Self {
        // Initialize GCP client
        let gcp_client = GcpClient::new(gcp_credentials_file, log.clone()).await;

        // Initialize registry manager
        let registry_manager = RegistryManager::new(log.clone(), targets_dir, vec![nns_url], poll_interval, registry_query_timeout);

        // Initialize the local registry
        if let Err(e) = registry_manager.initialize().await {
            error!(log, "Failed to initialize registry"; "error" => %e);
        } else {
            info!(log, "Registry initialized successfully");
        }

        // Perform initial sync with NNS
        info!(log, "Performing initial registry sync with NNS...");
        match registry_manager.sync().await {
            Ok(()) => info!(log, "Initial registry sync completed successfully"),
            Err(e) => warn!(log, "Initial registry sync failed (will retry)"; "error" => %e),
        }

        Self {
            log,
            config: Arc::new(config),
            gcp_client: Arc::new(gcp_client),
            registry_manager: Arc::new(registry_manager),
            subnet_proposals: Arc::new(DashMap::new()),
            poll_interval,
        }
    }

    /// Start the background registry sync loop
    /// Returns a sender that can be used to stop the sync loop
    pub fn start_registry_sync_loop(&self) -> watch::Sender<bool> {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let registry_manager = self.registry_manager.clone();
        let log = self.log.clone();
        let poll_interval = self.poll_interval;

        tokio::spawn(async move {
            info!(log, "Starting registry sync loop"; "interval" => ?poll_interval);

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(poll_interval) => {
                        info!(log, "Syncing registry with NNS...");
                        match registry_manager.sync().await {
                            Ok(()) => info!(log, "Registry sync completed"),
                            Err(e) => warn!(log, "Registry sync failed"; "error" => %e),
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            info!(log, "Registry sync loop stopped");
                            break;
                        }
                    }
                }
            }
        });

        stop_tx
    }
}
