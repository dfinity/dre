use ic_stable_structures::{storable::Bound, Storable};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::borrow::Cow;
use std::cell::RefCell;

use crate::types::{SubnetNodeMetrics, SubnetNodeMetricsStorable, TimestampNanos};

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

pub fn get_metrics(ts: TimestampNanos) -> Vec<(TimestampNanos, Vec<SubnetNodeMetrics>)> {
    MAP.with(|p| p.borrow().range(ts..).map(|(ts, storable)| (ts, storable.0)).collect_vec())
}
