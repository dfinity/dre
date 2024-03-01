use std::{path::PathBuf, time::Duration};

use ic_management_types::Network;
use service_discovery::{registry_sync::sync_local_registry, IcServiceDiscoveryImpl};
use slog::{debug, info, warn, Logger};
use tokio::{select, sync::broadcast::Receiver};

pub async fn inital_sync_wrap(
    logger: Logger,
    targets_dir: PathBuf,
    network: Network,
) -> anyhow::Result<InitSyncCompletionStatus> {
    let (_, stop_signal) = crossbeam::channel::bounded::<()>(0);

    let sync_fut = sync_local_registry(logger, targets_dir, vec![network.get_url()], false, None, &stop_signal);

    let shutdown_for_initial_sync = tokio::signal::ctrl_c();
    select! {
        res = sync_fut => {
            match res {
                Ok(_) => Ok(InitSyncCompletionStatus::Completed),
                Err(e) => Err(anyhow::anyhow!("{:?}", e))
            }
        }
        res = shutdown_for_initial_sync => match res {
            Ok(_) => Ok(InitSyncCompletionStatus::ShutdownRequested),
            Err(e) => Err(anyhow::anyhow!("{:?}", e))
        }
    }
}

pub enum InitSyncCompletionStatus {
    Completed,
    ShutdownRequested,
}

pub async fn poll(
    logger: Logger,
    targets_dir: PathBuf,
    registry_query_timeout: Duration,
    mut shutdown: Receiver<()>,
) -> anyhow::Result<()> {
    let disc = IcServiceDiscoveryImpl::new(logger.clone(), targets_dir, registry_query_timeout)
        .map_err(|e| anyhow::anyhow!("Couldn't create service discovery: {:?}", e))?;

    info!(logger, "Starting watching the network...");
    let mut interval = tokio::time::interval(registry_query_timeout);
    loop {
        select! {
            tick = interval.tick() => {
                debug!(logger, "Tick received, {:?}", tick);
            }
            _ = shutdown.recv() => {
                info!(logger, "Received shutdown request in 'poll' loop");
                return Ok(())
            }
        }

        debug!(logger, "Loading new targets...");
        if let Err(e) = disc.load_new_ics(logger.clone()) {
            warn!(logger, "Loading new targets failed! Error: {:?}", e)
        }
        debug!(logger, "Updating registries...");
        if let Err(e) = disc.update_registries().await {
            warn!(logger, "Updating registries failed! Error: {:?}", e)
        }
    }
}
