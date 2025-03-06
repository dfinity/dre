use std::path::PathBuf;

use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;
use slog::{error, info, warn, Logger};
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use super::{in_memory::InMemoryStorage, Storage};

#[derive(Clone)]
pub struct FileStorage {
    cache: InMemoryStorage,
    path: PathBuf,
    logger: Logger,
}

#[async_trait::async_trait]
impl Storage for FileStorage {
    async fn get(&self) -> anyhow::Result<Vec<multiservice_discovery_shared::contracts::journald_target::JournaldTarget>> {
        self.cache.get().await
    }

    async fn insert(&self, new_target: JournaldTarget) -> anyhow::Result<()> {
        let target_name = new_target.name.clone();
        info!(self.logger, "Trying to add new entry: {}", target_name);
        self.cache
            .insert(new_target)
            .await
            .map_err(|e| {
                error!(self.logger, "Failed to add new entry {} due to: {:?}", target_name, e);
                e
            })
            .inspect(|_| info!(self.logger, "Added entry {}", target_name))
    }

    async fn delete(&self, name: String) -> anyhow::Result<()> {
        info!(self.logger, "Trying to delete entry named: {}", name);
        self.cache
            .delete(name.clone())
            .await
            .map_err(|e| {
                error!(self.logger, "Failed to remove entry {} due to: {:?}", name, e);
                e
            })
            .inspect(|_| info!(self.logger, "Deleted entry {}", name))
    }

    fn sync(&self, handle: Handle, token: CancellationToken) -> JoinHandle<()> {
        let self_clone = self.clone();
        handle.spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {},
                    _ = token.cancelled() => {
                        info!(self_clone.logger, "Received shutdown in file storage sync");
                    }
                }

                info!(self_clone.logger, "Writing cache to disk...");
                if let Err(e) = self_clone.write().await {
                    error!(self_clone.logger, "Failed to sync cache to disk. Error: {:?}", e);
                } else {
                    info!(self_clone.logger, "Cache written to disk");
                }

                if token.is_cancelled() {
                    break;
                }
            }
        })
    }
}

impl FileStorage {
    pub fn new(path: PathBuf, logger: Logger) -> Self {
        let cache = match InMemoryStorage::try_from(path.as_path()) {
            Ok(from_disk) => from_disk,
            Err(e) => {
                warn!(logger, "Failed to create cache from disk: {:?}", e);
                warn!(logger, "Will create an empty one");
                InMemoryStorage::new()
            }
        };

        Self { cache, path, logger }
    }

    async fn write(&self) -> anyhow::Result<()> {
        let current_state = self.cache.get().await?;
        fs_err::write(&self.path, serde_json::to_string(&current_state).map_err(anyhow::Error::from)?).map_err(anyhow::Error::from)
    }
}
