use crate::metrics::TimestampNanos;
use candid::{CandidType, Decode, Encode, Principal};
use ic_base_types::{PrincipalId, SubnetId};
use ic_management_canister_types::NodeMetrics;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::borrow::Cow;
use std::ops::Deref;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 60;
const MAX_BYTES_NODE_METRICS_STORED: u32 = 76;
const PRINCIPAL_MAX_LENGTH_IN_BYTES: usize = 29;

pub const MIN_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[]));
pub const MAX_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[0xFF; PRINCIPAL_MAX_LENGTH_IN_BYTES]));

#[test]
fn max_bound_size() {
    let max_subnet_id_stored = SubnetIdStored(MAX_PRINCIPAL_ID.into());
    let max_node_metrics_stored_key = StorableSubnetMetricsKey {
        timestamp_nanos: u64::MAX,
        subnet_id: MAX_PRINCIPAL_ID.into(),
    };
    let max_node_metrics_stored = StorableSubnetMetrics(vec![NodeMetrics {
        node_id: MAX_PRINCIPAL_ID.into(),
        num_blocks_proposed_total: u64::MAX,
        num_block_failures_total: u64::MAX,
    }]);

    assert_eq!(max_subnet_id_stored.to_bytes().len(), MAX_BYTES_SUBNET_ID_STORED as usize);

    assert_eq!(max_node_metrics_stored_key.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED_KEY as usize);

    assert_eq!(max_node_metrics_stored.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED as usize);
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
pub struct StorableSubnetMetricsKey {
    pub timestamp_nanos: TimestampNanos,
    pub subnet_id: SubnetId,
}

impl Storable for StorableSubnetMetricsKey {
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
pub struct StorableSubnetMetrics(pub Vec<NodeMetrics>);

impl Storable for StorableSubnetMetrics {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        // This size supports subnets with max 400 nodes
        max_size: MAX_BYTES_NODE_METRICS_STORED * 400,
        is_fixed_size: false,
    };
}
