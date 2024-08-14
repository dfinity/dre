use candid::Principal;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::BTreeMap;

use trustworthy_node_metrics_types::types::{NodeMetricsStored, NodeMetricsStoredKey, TimestampNanos};

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {

    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static NODE_METRICS_MAP: RefCell<StableBTreeMap<NodeMetricsStoredKey, NodeMetricsStored, Memory>> =
      RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
    ));

}

pub fn insert(key: NodeMetricsStoredKey, value: NodeMetricsStored) {
    NODE_METRICS_MAP.with(|p| p.borrow_mut().insert(key, value));
}

pub fn latest_ts() -> Option<TimestampNanos> {
    NODE_METRICS_MAP.with(|p| p.borrow().last_key_value()).map(|((ts, _), _)| ts)
}

pub fn get_metrics_range(from_ts: TimestampNanos, to_ts: Option<TimestampNanos>) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    NODE_METRICS_MAP.with(|p| {
        let to_ts = to_ts.unwrap_or(u64::MAX);

        p.borrow().iter().filter(|((ts, _), _)| *ts >= from_ts && *ts <= to_ts).collect_vec()
    })
}

pub fn latest_metrics(principals: Vec<Principal>) -> BTreeMap<Principal, NodeMetricsStored> {
    let mut latest_metrics = BTreeMap::new();
    NODE_METRICS_MAP.with(|p| {
        for ((_, principal), value) in p.borrow().iter() {
            if principals.contains(&principal) {
                latest_metrics.insert(principal, value);
            }
        }
    });

    latest_metrics
}
