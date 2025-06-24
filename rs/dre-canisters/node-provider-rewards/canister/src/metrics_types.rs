use candid::{CandidType, Decode, Encode, Principal};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_management_canister_types_private::NodeMetrics;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use rewards_calculation::types::{NodeMetricsDailyRaw, SubnetMetricsDailyKey, UnixTsNanos};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 54;
const PRINCIPAL_MAX_LENGTH_IN_BYTES: usize = 29;

pub const MIN_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[]));
pub const MAX_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[0xFF_u8; PRINCIPAL_MAX_LENGTH_IN_BYTES]));

#[test]
fn max_bound_size() {
    let max_subnet_id_stored = SubnetIdKey(MAX_PRINCIPAL_ID.into());
    let max_subnet_metrics_stored_key = SubnetMetricsKeyStored {
        ts: u64::MAX,
        subnet_id: MAX_PRINCIPAL_ID.into(),
    };

    assert_eq!(max_subnet_id_stored.to_bytes().len(), MAX_BYTES_SUBNET_ID_STORED as usize);

    assert_eq!(max_subnet_metrics_stored_key.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED_KEY as usize);
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubnetIdKey(pub SubnetId);
impl Storable for SubnetIdKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_SUBNET_ID_STORED,
        is_fixed_size: false,
    };
}

impl From<SubnetId> for SubnetIdKey {
    fn from(subnet_id: SubnetId) -> Self {
        Self(subnet_id)
    }
}

pub trait KeyRange {
    fn min_key() -> Self;
    fn max_key() -> Self;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubnetMetricsKeyStored {
    pub ts: UnixTsNanos,
    pub subnet_id: SubnetId,
}

impl From<SubnetMetricsKeyStored> for SubnetMetricsDailyKey {
    fn from(key: SubnetMetricsKeyStored) -> Self {
        Self {
            day: key.ts.into(),
            subnet_id: key.subnet_id,
        }
    }
}

impl Storable for SubnetMetricsKeyStored {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(serde_cbor::to_vec(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_NODE_METRICS_STORED_KEY,
        is_fixed_size: false,
    };
}

impl KeyRange for SubnetMetricsKeyStored {
    fn min_key() -> Self {
        Self {
            ts: u64::MIN,
            subnet_id: MIN_PRINCIPAL_ID.into(),
        }
    }

    fn max_key() -> Self {
        Self {
            ts: u64::MAX,
            subnet_id: MAX_PRINCIPAL_ID.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetricsStored {
    pub node_id: NodeId,
    pub num_blocks_proposed_total: u64,
    pub num_blocks_failed_total: u64,
}

impl From<NodeMetrics> for NodeMetricsStored {
    fn from(node_metrics: NodeMetrics) -> Self {
        Self {
            node_id: NodeId::from(node_metrics.node_id),
            num_blocks_proposed_total: node_metrics.num_blocks_proposed_total,
            num_blocks_failed_total: node_metrics.num_block_failures_total,
        }
    }
}

impl From<NodeMetricsStored> for NodeMetricsDailyRaw {
    fn from(node_metrics: NodeMetricsStored) -> Self {
        Self {
            node_id: node_metrics.node_id,
            num_blocks_proposed: node_metrics.num_blocks_proposed_total,
            num_blocks_failed: node_metrics.num_blocks_failed_total,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubnetMetricsValueStored {
    pub nodes_metrics: Vec<NodeMetricsStored>,
}

impl From<SubnetMetricsValueStored> for Vec<NodeMetricsDailyRaw> {
    fn from(subnet_metrics: SubnetMetricsValueStored) -> Self {
        subnet_metrics.nodes_metrics.into_iter().map(NodeMetricsDailyRaw::from).collect()
    }
}

impl Storable for SubnetMetricsValueStored {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(serde_cbor::to_vec(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}
