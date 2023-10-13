use actix_web::web;
use ic_management_backend::registry::{self, RegistryState};
use ic_management_types::Network;

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::ServiceHealth;

pub struct RegistryLoopConfig {
    pub cancellation_token: CancellationToken,
    pub target_network: Network,
    pub service_health: web::Data<ServiceHealth>,
}

// There is no real information in the Config, so just print its name as debug
// Necessary for tracing
impl std::fmt::Debug for RegistryLoopConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "RegistryLoopConfig")
    }
}

#[tracing::instrument]
pub async fn start_registry_updater_loop(config: RegistryLoopConfig) {
    // Having a way to know where we are regarding the update
    // of the registry would be good. The service takes a long time to start,
    // and we want to get some information about startup if possible
    info!("Starting Registry Updater loop");
    loop {
        if config.cancellation_token.is_cancelled() {
            break;
        }
        config.service_health.set_registry_updater_loop_readiness(true);
        if let Err(e) = registry::sync_local_store(config.target_network.clone()).await {
            error!(message = "Failed to update local registry", error = e.to_string());
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

pub async fn create_registry_state() -> RegistryState {
    let target_network = ic_management_backend::config::target_network();
    ic_management_backend::registry::sync_local_store(target_network.clone())
        .await
        .expect("failed to init local store");

    RegistryState::new(ic_management_types::Network::Mainnet, true).await
}
