use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::cell::RefCell;
use trustworthy_node_metrics_types::types::{SubnetNodeMetrics, SubnetNodeMetricsStorable, TimestampNanos};

thread_local! {
    pub static MAP: RefCell<StableBTreeMap<TimestampNanos, SubnetNodeMetricsStorable, DefaultMemoryImpl>> =
      RefCell::new(StableBTreeMap::init(DefaultMemoryImpl::default()));
}

pub fn insert(key: TimestampNanos, value: Vec<SubnetNodeMetrics>) {
    MAP.with(|p| p.borrow_mut().insert(key, SubnetNodeMetricsStorable(value)));
}

pub fn latest_key() -> Option<TimestampNanos> {
    MAP.with(|p| p.borrow().last_key_value()).map(|(k, _)| k)
}

pub fn get_metrics(ts: TimestampNanos) -> Vec<(TimestampNanos, Vec<SubnetNodeMetrics>)> {
    MAP.with(|p| p.borrow().range(ts..).map(|(ts, storable)| (ts, storable.0)).collect_vec())
}

pub fn get_nodes() -> Vec<String> {
    MAP.with(|p| p.borrow().iter().collect_vec())
        .into_iter().flat_map(|(_, storable)| {
            storable.0.into_iter().map(|s| s.node_metrics.into_iter().map(|n| n.node_id.to_string()).collect_vec())
        }).flatten().collect_vec()
}
