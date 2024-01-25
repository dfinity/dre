use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::TargetGroup;

/// Record of the shape as described in
/// https://prometheus.io/docs/prometheus/latest/http_sd/
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ServiceDiscoveryRecord {
    targets: Vec<String>,             // targets: ["ip:port"]
    labels: BTreeMap<String, String>, // labels: { k: v, k : v}
}

impl From<TargetGroup> for ServiceDiscoveryRecord {
    fn from(group: TargetGroup) -> Self {
        Self {
            targets: group.targets.into_iter().map(|x| x.to_string()).collect(),
            labels: BTreeMap::from([
                (IC_NAME.into(), group.ic_name),
                (IC_NODE.into(), group.node_id.to_string()),
            ])
            .into_iter()
            .chain(match group.subnet_id {
                Some(subnet_id) => vec![(IC_SUBNET.into(), subnet_id.to_string())],
                None => vec![],
            })
            .collect(),
        }
    }
}

// Default labels
const IC_NAME: &str = "ic";
const IC_NODE: &str = "ic_node";
const IC_SUBNET: &str = "ic_subnet";
