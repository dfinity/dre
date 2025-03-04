use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;
use slog::{info, warn, Logger};
use tokio::{net::TcpStream, runtime::Handle, sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{metrics::Metrics, storage::Storage};

#[derive(Clone)]
pub struct TargetSupervisor {
    logger: Logger,
    token: CancellationToken,
    metrics: Metrics,
    storage: Arc<dyn Storage>,
    handle: Handle,
    running_targets: Arc<Mutex<BTreeMap<String, JoinHandle<()>>>>,
}

impl TargetSupervisor {
    pub fn new(logger: Logger, token: CancellationToken, metrics: Metrics, storage: Arc<dyn Storage>, handle: Handle) -> Self {
        Self {
            logger,
            token,
            metrics,
            storage,
            handle,
            running_targets: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub async fn start_cached_targets(&self) {
        for target in self.storage.get().await.unwrap() {
            self.run_target(target).await.unwrap();
        }
    }

    async fn run_target(&self, target: JournaldTarget) -> anyhow::Result<()> {
        let mut running_targets = self.running_targets.lock().await;
        if let Some(_) = running_targets.get(&target.name) {
            return Err(anyhow::anyhow!("Definition with the name {} already running", target.name));
        }

        let target_name = target.name.clone();
        let running_target = RunningTarget::new(self.logger.clone(), self.metrics.clone(), target, self.token.clone());

        let join_handle = self.handle.spawn(running_target.poll());

        running_targets.insert(target_name, join_handle);
        Ok(())
    }

    pub async fn stop_cached_targets(&self) {
        let mut current_running_targets = self.running_targets.lock().await;
        for (name, target) in current_running_targets.iter_mut() {
            target.await.unwrap_or_else(|_| panic!("Failed to join running target {}", name));
        }
    }
}

#[async_trait::async_trait]
impl Storage for TargetSupervisor {
    async fn get(&self) -> anyhow::Result<Vec<JournaldTarget>> {
        self.storage.get().await
    }

    async fn upsert(&self, new_targets: Vec<JournaldTarget>) -> anyhow::Result<()> {
        // TODO: Should fix the checking so that the batch insert succeeds
        self.storage.upsert(new_targets.clone()).await?;
        for target in new_targets.into_iter() {
            self.run_target(target).await?
        }

        Ok(())
    }

    async fn delete(&self, names: Vec<String>) -> anyhow::Result<()> {
        self.storage.delete(names).await
    }

    fn sync(&self, _handle: Handle, _token: CancellationToken) -> JoinHandle<()> {
        unreachable!("Shouldn't happen. This trait is implemented as a convenience for the server")
    }
}

struct RunningTarget {
    logger: Logger,
    token: CancellationToken,
    metrics: Metrics,
    target: JournaldTarget,
}

impl RunningTarget {
    fn new(logger: Logger, metrics: Metrics, target: JournaldTarget, token: CancellationToken) -> Self {
        Self {
            logger,
            metrics,
            token,
            target,
        }
    }

    async fn poll(self) {
        // Poll targets each 10 seconds to see if they are reachable
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

        self.info("Starting watching target");
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                _ = self.token.cancelled() => {
                    self.info("Received shutdown request");
                    break;
                }
            }

            match TcpStream::connect(self.target.target).await {
                Ok(_) => {
                    self.metrics.observe_up(&self.target.name);
                }
                Err(e) => {
                    self.warn(format!("Target {} unreachable: {:?}", self.target.target, e));
                    self.metrics.observe_down(&self.target.name);
                }
            }
        }
    }

    fn format_message<A: AsRef<str> + Display>(&self, message: A) -> String {
        format!("[{}]: {}", self.target.name, message)
    }

    fn info<A: AsRef<str> + Display>(&self, message: A) {
        info!(self.logger, "{}", self.format_message(message))
    }

    fn warn<A: AsRef<str> + Display>(&self, message: A) {
        warn!(self.logger, "{}", self.format_message(message))
    }
}
