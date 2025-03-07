use candid::{CandidType, Decode, Encode};
use ic_base_types::SubnetId;
use ic_management_canister_types::NodeMetrics;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::Deserialize;
use std::borrow::Cow;

pub type TimestampNanos = u64;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 60;
const MAX_BYTES_NODE_METRICS_STORED: u32 = 76;

#[test]
fn max_bound_size() {
    use candid::Principal;
    use ic_base_types::PrincipalId;

    let max_principal_id = PrincipalId::from(Principal::from_slice(&[0xFF; 29]));

    let max_subnet_id_stored = SubnetIdStored(max_principal_id.into());
    let max_node_metrics_stored_key = SubnetMetricsStoredKey {
        timestamp_nanos: u64::MAX,
        subnet_id: max_principal_id.into(),
    };
    let max_node_metrics_stored = SubnetMetricsStored(vec![NodeMetrics {
        node_id: max_principal_id,
        num_blocks_proposed_total: u64::MAX,
        num_block_failures_total: u64::MAX,
    }]);

    assert_eq!(max_subnet_id_stored.to_bytes().len(), MAX_BYTES_SUBNET_ID_STORED as usize);

    assert_eq!(max_node_metrics_stored_key.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED_KEY as usize);

    assert_eq!(max_node_metrics_stored.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED as usize);
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SubnetIdStored(pub(crate) SubnetId);
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

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SubnetMetricsStoredKey {
    pub timestamp_nanos: TimestampNanos,
    pub subnet_id: SubnetId,
}

impl Storable for SubnetMetricsStoredKey {
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
pub(crate) struct SubnetMetricsStored(pub(crate) Vec<NodeMetrics>);

impl Storable for SubnetMetricsStored {
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
