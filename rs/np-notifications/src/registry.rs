use std::{sync::Arc, time::Duration};

use ic_management_backend::registry::{self, RegistryState};
use ic_management_types::Network;
use ic_registry_local_registry::LocalRegistry;

use slog::Logger;
use tokio_util::sync::CancellationToken;

pub struct RegistryLoopConfig {
    pub logger: Logger,
    pub cancellation_token: CancellationToken,
    pub target_network: Network,
}

pub async fn start_registry_updater_loop(config: RegistryLoopConfig) {
    // Having a way to know where we are regarding the update
    // of the registry would be good. The service takes a long time to start,
    // and we want to get some information about startup if possible
    let log = config.logger.clone();
    info!(log, "Starting Registry Updater loop");
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        if let Err(e) = registry::sync_local_store(config.target_network.clone()).await {
            error!(log, "Failed to update local registry: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

pub async fn create_registry_state() -> RegistryState {
    let target_network = ic_management_backend::config::target_network();
    ic_management_backend::registry::sync_local_store(target_network.clone())
        .await
        .expect("failed to init local store");

    let local_registry_path = ic_management_backend::registry::local_registry_path(target_network.clone());
    let local_registry: Arc<LocalRegistry> = Arc::new(
        LocalRegistry::new(local_registry_path, Duration::from_millis(1000)).expect("Failed to create local registry"),
    );

    RegistryState::new(
        "https://ic0.app".to_string(),
        ic_management_types::Network::Mainnet,
        local_registry.clone(),
        None,
    )
}
