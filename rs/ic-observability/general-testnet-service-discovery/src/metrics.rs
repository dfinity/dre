use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use opentelemetry::{global, metrics::Observer, KeyValue};

#[derive(Clone, Default)]
pub struct Values {
    target_status: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct Metrics {
    latest_values: Arc<RwLock<Values>>,
}

impl Metrics {
    pub fn new() -> Self {
        let latest_values = Arc::new(RwLock::new(Values::default()));
        let meter = global::meter("axum-app");

        let total_targets = meter
            .clone()
            .u64_observable_gauge("gsd.total_targets")
            .with_description("Total number of targets present on the general service discovery")
            .init();
        let target_status = meter
            .clone()
            .u64_observable_gauge("gsd.target_up")
            .with_description("Resembles the UP metric from prometheus for known targets")
            .init();

        let instruments = [total_targets.as_any(), target_status.as_any()];
        let values_clone = latest_values.clone();
        let update_instruments = move |observer: &dyn Observer| {
            let values = values_clone.read().unwrap();
            for (instrument, measurement) in [(&total_targets, values.target_status.len())].into_iter() {
                observer.observe_u64(instrument, measurement as u64, &[]);
            }

            for (target, up) in &values.target_status {
                let attrs = [KeyValue::new("name", target.clone())];
                observer.observe_u64(&target_status, *up, &attrs);
            }
        };

        meter.register_callback(&instruments, update_instruments).unwrap();
        Self { latest_values }
    }

    fn observe_status(&self, name: &str, status: u64) {
        let mut values = self.latest_values.write().unwrap();

        let latest_status = values.target_status.entry(name.to_string()).or_insert(0);

        *latest_status = status;
    }

    pub fn observe_up(&self, name: &str) {
        self.observe_status(name, 1);
    }

    pub fn observe_down(&self, name: &str) {
        self.observe_status(name, 0);
    }

    pub fn remove_observed_value(&self, name: &str) {
        let mut values = self.latest_values.write().unwrap();

        values.target_status.remove(name);
    }
}
