use std::path::PathBuf;

use ic_management_types::Network;
use service_discovery::registry_sync::sync_local_registry;
use slog::Logger;

pub async fn sync_wrap(logger: Logger, targets_dir: PathBuf, network: Network) -> anyhow::Result<()> {
    let (_, stop_signal) = crossbeam::channel::bounded::<()>(0);

    sync_local_registry(logger, targets_dir, vec![network.get_url()], false, None, &stop_signal)
        .await
        .map_err(|e| anyhow::anyhow!("Error during syncing registry: {:?}", e))
}
