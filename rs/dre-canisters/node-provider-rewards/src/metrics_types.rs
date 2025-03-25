use crate::metrics::TimestampNanos;
use candid::{CandidType, Decode, Encode, Principal};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_management_canister_types::NodeMetrics;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::borrow::Cow;
use std::i8::MIN;
use std::ops::Deref;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_NODE_ID_STORED: u32 = 38;
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 95;
const MAX_BYTES_NODE_METRICS_STORED_VALUE: u32 = 73;

const PRINCIPAL_MAX_LENGTH_IN_BYTES: usize = 29;

// For range scanning.
lazy_static! {
    pub static ref MIN_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::try_from(vec![]).expect("Unable to construct MIN_PRINCIPAL_ID."));
    pub static ref MAX_PRINCIPAL_ID: PrincipalId =
        PrincipalId(Principal::try_from(vec![0xFF_u8; PRINCIPAL_MAX_LENGTH_IN_BYTES]).expect("Unable to construct MAX_PRINCIPAL_ID."));
}

#[test]
fn max_bound_size() {
    use candid::Principal;
    use ic_base_types::PrincipalId;

    let max_principal_id = PrincipalId::from(Principal::from_slice(&[0xFF; 29]));

    let max_node_id_stored = NodeIdStored(max_principal_id.into());
    let max_daily_node_metrics_stored_key = StorableNodeMetricsKey {
        ts: u64::MAX,
        node_id: max_principal_id.into(),
        subnet_assigned: max_principal_id.into(),
    };
    let max_daily_node_metrics_stored_value = StorableNodeMetrics(NodeMetrics {
        node_id: max_principal_id,
        num_blocks_proposed_total: u64::MAX,
        num_block_failures_total: u64::MAX,
    });

    assert_eq!(max_node_id_stored.to_bytes().len(), MAX_BYTES_NODE_ID_STORED as usize);

    assert_eq!(
        max_daily_node_metrics_stored_key.to_bytes().len(),
        MAX_BYTES_NODE_METRICS_STORED_KEY as usize
    );

    assert_eq!(
        max_daily_node_metrics_stored_value.to_bytes().len(),
        MAX_BYTES_NODE_METRICS_STORED_VALUE as usize
    );
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeIdStored(pub NodeId);
impl Storable for NodeIdStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_NODE_ID_STORED,
        is_fixed_size: false,
    };
}

impl Deref for NodeIdStored {
    type Target = NodeId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NodeId> for NodeIdStored {
    fn from(node_id: NodeId) -> Self {
        Self(node_id)
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubnetIdStored(pub SubnetId);
impl Storable for SubnetIdStored {
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

impl Deref for SubnetIdStored {
    type Target = SubnetId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SubnetId> for SubnetIdStored {
    fn from(subnet_id: SubnetId) -> Self {
        Self(subnet_id)
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorableNodeMetricsKey {
    pub ts: TimestampNanos,
    pub node_id: NodeId,
    pub subnet_assigned: SubnetId,
}

impl Default for StorableNodeMetricsKey {
    fn default() -> Self {
        Self {
            ts: u64::MIN,
            node_id: NodeId::from(MIN_PRINCIPAL_ID.clone()),
            subnet_assigned: SubnetId::from(MIN_PRINCIPAL_ID.clone()),
        }
    }
}

impl Storable for StorableNodeMetricsKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_NODE_METRICS_STORED_KEY,
        is_fixed_size: false,
    };
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct StorableNodeMetrics(pub NodeMetrics);

impl Storable for StorableNodeMetrics {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_NODE_METRICS_STORED_VALUE,
        is_fixed_size: false,
    };
}
