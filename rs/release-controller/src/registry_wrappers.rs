use std::{path::PathBuf, time::Duration};

use ic_management_types::Network;
use service_discovery::{registry_sync::sync_local_registry, IcServiceDiscoveryImpl};
use slog::{debug, info, warn, Logger};
use tokio::{runtime::Handle, select};

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

pub fn poll(
    logger: Logger,
    targets_dir: PathBuf,
    registry_query_timeout: Duration,
    shutdown: crossbeam::channel::Receiver<()>,
    rt: Handle,
) -> anyhow::Result<()> {
    let disc = IcServiceDiscoveryImpl::new(logger.clone(), targets_dir, registry_query_timeout)
        .map_err(|e| anyhow::anyhow!("Couldn't create service discovery: {:?}", e))?;

    info!(logger, "Starting watching the network...");
    let interval = crossbeam::channel::tick(registry_query_timeout);
    loop {
        crossbeam::select! {
            recv(shutdown) -> _ => return Ok(()),
            recv(interval) -> tick => {
                debug!(logger, "Received tick {:?}", tick.unwrap())
            }
        }

        debug!(logger, "Loading new targets...");
        if let Err(e) = disc.load_new_ics(logger.clone()) {
            warn!(logger, "Loading new targets failed! Error: {:?}", e)
        }
        debug!(logger, "Updating registries...");
        if let Err(e) = rt.block_on(disc.update_registries()) {
            warn!(logger, "Updating registries failed! Error: {:?}", e)
        }
    }
}
