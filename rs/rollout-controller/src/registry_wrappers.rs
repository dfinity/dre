use std::path::PathBuf;

use ic_management_backend::{git_ic_repo::IcRepo, registry::RegistryState};
use ic_management_types::Network;
use service_discovery::registry_sync::sync_local_registry;
use slog::{debug, Logger};

pub async fn sync_wrap(logger: Logger, targets_dir: PathBuf, network: Network) -> anyhow::Result<RegistryState> {
    let (_, stop_signal) = crossbeam::channel::bounded::<()>(0);

    sync_local_registry(logger.clone(), targets_dir, network.get_nns_urls(), false, None, &stop_signal)
        .await
        .map_err(|e| anyhow::anyhow!("Error during syncing registry: {:?}", e))?;

    // Check if the desired rollout version is elected
    debug!(logger, "Creating registry");
    let mut registry_state = RegistryState::new(&network, true, Some(IcRepo::new().expect("Should be able to create IC repo"))).await;

    debug!(logger, "Updating registry with data");
    let node_provider_data = vec![];
    registry_state.update_node_details(&node_provider_data).await?;
    debug!(logger, "Created registry with latest version: '{}'", registry_state.version());

    Ok(registry_state)
}
