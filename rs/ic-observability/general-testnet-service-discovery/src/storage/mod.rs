use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub mod file;
pub mod in_memory;

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self) -> anyhow::Result<Vec<JournaldTarget>>;

    async fn insert(&self, new_target: JournaldTarget) -> anyhow::Result<()>;

    async fn delete(&self, name: String) -> anyhow::Result<()>;

    fn sync(&self, handle: Handle, token: CancellationToken) -> JoinHandle<()>;
}
