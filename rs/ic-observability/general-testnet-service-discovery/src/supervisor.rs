use std::{
    collections::BTreeMap,
    fmt::Display,
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
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
    gc_timeout: Duration,
    check_interval: Duration,
}

impl TargetSupervisor {
    pub fn new(
        logger: Logger,
        token: CancellationToken,
        metrics: Metrics,
        storage: Arc<dyn Storage>,
        handle: Handle,
        gc_timeout: Duration,
        check_interval: Duration,
    ) -> Self {
        Self {
            logger,
            token,
            metrics,
            storage,
            handle,
            running_targets: Arc::new(Mutex::new(BTreeMap::new())),
            gc_timeout,
            check_interval,
        }
    }

    pub async fn start_cached_targets(&self) {
        for target in self.storage.get().await.unwrap() {
            self.run_target(target).await.unwrap();
        }
    }

    async fn run_target(&self, target: JournaldTarget) -> anyhow::Result<()> {
        let mut running_targets = self.running_targets.lock().await;
        if running_targets.get(&target.name).is_some() {
            return Err(anyhow::anyhow!("Definition with the name {} already running", target.name));
        }

        let target_name = target.name.clone();

        let target_name_for_remove = target.name.clone();
        let storage_clone = self.storage.clone();
        let remove_target = move || async move { storage_clone.delete(target_name_for_remove).await.unwrap() };
        let running_target = RunningTarget::new(
            self.logger.clone(),
            self.metrics.clone(),
            target,
            self.token.clone(),
            remove_target,
            self.gc_timeout,
            self.check_interval,
        );

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

    async fn insert(&self, new_target: JournaldTarget) -> anyhow::Result<()> {
        self.storage.insert(new_target.clone()).await?;
        self.run_target(new_target).await
    }

    async fn delete(&self, name: String) -> anyhow::Result<()> {
        self.storage.delete(name).await
    }
}

struct RunningTarget<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    logger: Logger,
    token: CancellationToken,
    metrics: Metrics,
    target: JournaldTarget,
    last_successful_sync: Instant,
    gc_timeout: Duration,
    remove_self: F,
    check_interval: Duration,
}

impl<F, Fut> RunningTarget<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    fn new(
        logger: Logger,
        metrics: Metrics,
        target: JournaldTarget,
        token: CancellationToken,
        remove_self: F,
        gc_timeout: Duration,
        check_interval: Duration,
    ) -> Self {
        Self {
            logger,
            metrics,
            token,
            target,
            last_successful_sync: Instant::now(),
            gc_timeout,
            remove_self,
            check_interval,
        }
    }

    async fn poll(mut self) {
        let mut interval = tokio::time::interval(self.check_interval);

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
                    self.last_successful_sync = Instant::now();
                }
                Err(e) => {
                    let elapsed_from_last_sync = self.last_successful_sync.elapsed();
                    let datetime: DateTime<Utc> = Utc::now() - elapsed_from_last_sync;
                    self.warn(format!(
                        "Target {} unreachable: {:?}, last successful sync: {}",
                        self.target.target,
                        e,
                        datetime.to_rfc3339()
                    ));
                    self.metrics.observe_down(&self.target.name);
                    if elapsed_from_last_sync > self.gc_timeout {
                        self.info("GC elapsed, removing target...");
                        (self.remove_self)().await;
                        self.metrics.remove_observed_value(&self.target.name);
                        break;
                    }
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
