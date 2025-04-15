use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use opentelemetry::{global, metrics::AsyncInstrument, KeyValue};

#[derive(Clone, Default)]
pub struct Values {
    target_status: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct Metrics {
    latest_values: Arc<RwLock<Values>>,
}

fn create_callback_total_targets(latest_values: Arc<RwLock<Values>>) -> impl Fn(&dyn AsyncInstrument<u64>) {
    move |observer: &dyn AsyncInstrument<u64>| {
        let latest_values = latest_values.read().unwrap();
        observer.observe(latest_values.target_status.len() as u64, &[]);
    }
}
fn create_callback_target_status(latest_values: Arc<RwLock<Values>>) -> impl Fn(&dyn AsyncInstrument<u64>) {
    move |observer: &dyn AsyncInstrument<u64>| {
        let latest_values = latest_values.read().unwrap();

        for (target, up) in &latest_values.target_status {
            let attrs = [KeyValue::new("name", target.clone())];
            observer.observe(*up, &attrs);
        }
    }
}

impl Metrics {
    pub fn new() -> Self {
        let latest_values = Arc::new(RwLock::new(Values::default()));
        let meter = global::meter("axum-app");

        let _total_targets = meter
            .u64_observable_gauge("gsd.total_targets")
            .with_description("Total number of targets present on the general service discovery")
            .with_callback(create_callback_total_targets(latest_values.clone()))
            .build();
        let _target_status = meter
            .u64_observable_gauge("gsd.target_up")
            .with_description("Resembles the UP metric from prometheus for known targets")
            .with_callback(create_callback_target_status(latest_values.clone()))
            .build();

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
