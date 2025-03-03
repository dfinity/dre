use std::{collections::BTreeMap, hash::Hash, net::SocketAddr};

use serde::{Deserialize, Serialize};

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
