use crate::metrics::TimestampNanos;
use candid::{CandidType, Decode, Encode, Principal};
use ic_base_types::{PrincipalId, SubnetId};
use ic_management_canister_types::NodeMetrics;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::Deserialize;
use std::borrow::Cow;
use std::ops::Deref;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 60;
const PRINCIPAL_MAX_LENGTH_IN_BYTES: usize = 29;

pub const MIN_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[]));
pub const MAX_PRINCIPAL_ID: PrincipalId = PrincipalId(Principal::from_slice(&[0xFF; PRINCIPAL_MAX_LENGTH_IN_BYTES]));

#[test]
fn max_bound_size() {
    let max_subnet_id_stored = SubnetIdStored(MAX_PRINCIPAL_ID.into());
    let max_subnet_metrics_stored_key = StorableSubnetMetricsKey {
        ts: u64::MAX,
        subnet_id: MAX_PRINCIPAL_ID.into(),
    };

    assert_eq!(max_subnet_id_stored.to_bytes().len(), MAX_BYTES_SUBNET_ID_STORED as usize);

    assert_eq!(max_subnet_metrics_stored_key.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED_KEY as usize);
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

pub trait KeyRange {
    fn min_key() -> Self;
    fn max_key() -> Self;
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorableSubnetMetricsKey {
    pub ts: TimestampNanos,
    pub subnet_id: SubnetId,
}

impl KeyRange for StorableSubnetMetricsKey {
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

    const BOUND: Bound = Bound::Unbounded;
}

impl Deref for StorableSubnetMetrics {
    type Target = Vec<NodeMetrics>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
