use std::{collections::BTreeMap, hash::Hash, net::SocketAddr};

use serde::{Deserialize, Serialize};

use super::DataContract;

/// Any target that has systemd-journal-gatewayd exposed.
/// Used for scraping testnet targets
#[derive(Clone, Serialize, Deserialize)]
pub struct JournaldTarget {
    pub name: String,
    pub target: SocketAddr,
    pub labels: BTreeMap<String, String>,
}

impl Hash for JournaldTarget {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.target.hash(state);
        self.labels.hash(state);
    }
}

impl DataContract for JournaldTarget {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_id(&self) -> String {
        self.name.clone()
    }

    fn get_target_name(&self) -> String {
        self.name.clone()
    }
}
