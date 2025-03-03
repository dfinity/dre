use std::{collections::BTreeMap, path::Path, sync::Arc};

use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;
use tokio::{runtime::Handle, sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use super::Storage;

/// Implementation used for testing and local
#[derive(Clone)]
pub struct InMemoryStorage {
    targets: Arc<RwLock<BTreeMap<String, JournaldTarget>>>,
}

#[async_trait::async_trait]
impl Storage for InMemoryStorage {
    async fn get(&self) -> anyhow::Result<Vec<JournaldTarget>> {
        let targets = self.targets.read().await;

        Ok(targets.values().cloned().collect())
    }

    async fn upsert(&self, new_targets: Vec<JournaldTarget>) -> anyhow::Result<()> {
        let mut targets = self.targets.write().await;

        for new_target in new_targets {
            targets.insert(new_target.name.clone(), new_target);
        }

        Ok(())
    }

    async fn delete(&self, names: Vec<String>) -> anyhow::Result<()> {
        let mut targets = self.targets.write().await;

        targets.retain(|name, _| !names.contains(name));

        Ok(())
    }

    fn sync(&self, handle: Handle, _token: CancellationToken) -> JoinHandle<()> {
        handle.spawn(async {})
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            targets: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl TryFrom<&Path> for InMemoryStorage {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let content = fs_err::read_to_string(&value).map_err(|e| anyhow::anyhow!("Failed to read file {}: {:?}", value.display(), e))?;

        let deserialized: Vec<JournaldTarget> =
            serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to deserialize the stored value: {:?}", e))?;

        Ok(Self {
            targets: Arc::new(RwLock::new(
                deserialized.iter().map(|target| (target.name.clone(), target.clone())).collect(),
            )),
        })
    }
}
