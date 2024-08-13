use candid::{Decode, Encode, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::borrow::Cow;
use std::cell::RefCell;

use crate::types::{NodeMetricsStored, NodeMetricsStoredKey, TimestampNanos};

impl Storable for NodeMetricsStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

thread_local! {
    pub static MAP: RefCell<StableBTreeMap<NodeMetricsStoredKey, NodeMetricsStored, DefaultMemoryImpl>> =
      RefCell::new(StableBTreeMap::init(DefaultMemoryImpl::default()));
}

pub fn insert(key: NodeMetricsStoredKey, value: NodeMetricsStored) {
    MAP.with(|p| p.borrow_mut().insert(key, value));
}

pub fn latest_ts() -> Option<TimestampNanos> {
    MAP.with(|p| p.borrow().last_key_value()).map(|((ts, _), _)| ts)
}

#[allow(dead_code)]
pub fn get(node_metrics_key: &NodeMetricsStoredKey) -> Option<NodeMetricsStored> {
    MAP.with(|p| p.borrow().get(node_metrics_key))
}

pub fn get_metrics_range(from_ts: TimestampNanos, to_ts: Option<TimestampNanos>) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    let range = {
        let first = (from_ts, Principal::anonymous());
        let last = (to_ts.unwrap_or(u64::MAX), Principal::anonymous());
        first..=last
    };

    MAP.with(|p| p.borrow().range(range).collect_vec())
}

pub fn metrics_before_ts(principal: Principal, ts: u64) -> Option<(NodeMetricsStoredKey, NodeMetricsStored)> {
    MAP.with(|p| {
        p.borrow()
            .range((u64::MIN, principal)..(ts, principal))
            .filter(|((_, p), _)| p == &principal)
            .last()
    })
}
