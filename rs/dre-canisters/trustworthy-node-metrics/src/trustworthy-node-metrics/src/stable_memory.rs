use candid::Principal;
use ic_stable_structures::{storable::Bound, Storable};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::types::{NodeMetrics, SubnetNodeMetrics, SubnetNodeMetricsStorable, TimestampNanos};

impl Storable for SubnetNodeMetricsStorable {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        ciborium::ser::into_writer(self, &mut buf).expect("failed to encode SubnetsMetricsStorable");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        ciborium::de::from_reader(&bytes[..]).expect("failed to decode SubnetsMetricsStorable")
    }

    const BOUND: Bound = Bound::Unbounded;
}

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

pub fn get_metrics_range(from_ts: TimestampNanos, to_ts: Option<TimestampNanos>) -> Vec<(TimestampNanos, Vec<SubnetNodeMetrics>)> {
    return MAP.with(|p| {
        let borrowed = p.borrow();
        let metrics_in_range = match to_ts {
            Some(to_ts) => borrowed.range(from_ts..=to_ts),
            None => borrowed.range(from_ts..),
        };
        metrics_in_range.into_iter().map(|(ts, storable)| (ts, storable.0)).collect_vec()
    });
}

// Initialize metrics
pub fn metrics_before_ts(node_ids: Vec<Principal>, ts: &TimestampNanos) -> BTreeMap<Principal, NodeMetrics> {
    let mut last_metrics: BTreeMap<Principal, NodeMetrics> = BTreeMap::new();

    // Initialize nodes
    for node_id in node_ids {
        last_metrics.insert(node_id, NodeMetrics{
            node_id,
            num_block_failures_total: 0,
            num_blocks_proposed_total: 0
        });
    }

    MAP.with(|p| {
        let borrowed = p.borrow();

        for (ts, metrics_storable) in borrowed.range(..ts) {
            for subnet_metrics in metrics_storable.0 {
                for node_metrics in subnet_metrics.node_metrics {
                    ic_cdk::println!("initial ts node {} {}", node_metrics.node_id, ts);
                    last_metrics.insert(node_metrics.node_id, node_metrics);
                }
            }
        }
    });

    last_metrics
}
