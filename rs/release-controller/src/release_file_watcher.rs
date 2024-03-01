use slog::{debug, info, Logger};
use tokio::{select, sync::broadcast::Receiver};

pub async fn handle_file_update(
    logger: Logger,
    mut receiver: Receiver<()>,
    mut shutdown: Receiver<()>,
) -> anyhow::Result<()> {
    loop {
        debug!(logger, "Waiting for events from file system to come");
        select! {
            _ = receiver.recv() => {
                info!(logger, "File changed!");
            }
            _ = shutdown.recv() => {
                info!(logger, "Received shutdown signal in 'file update' loop");
                return Ok(())
            }
        }
    }
}
