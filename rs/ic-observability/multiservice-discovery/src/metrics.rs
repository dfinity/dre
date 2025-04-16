use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use opentelemetry::{global, metrics::AsyncInstrument, KeyValue};

const NETWORK: &str = "network";
const AXUM_APP: &str = "axum_otel_metrics";

#[derive(Clone)]
pub struct MSDMetrics {
    pub running_definition_metrics: RunningDefinitionsMetrics,
}

impl Default for MSDMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MSDMetrics {
    pub fn new() -> Self {
        Self {
            running_definition_metrics: RunningDefinitionsMetrics::new(),
        }
    }
}

struct LatestValues {
    load_new_targets_error: u64,
    sync_registry_error: u64,
    definitions_load_successful: u64,
    definitions_sync_successful: u64,
    successful_sync_ts: u64,
}

impl LatestValues {
    fn new() -> Self {
        Self {
            load_new_targets_error: 0,
            sync_registry_error: 0,
            definitions_load_successful: 0,
            definitions_sync_successful: 0,
            successful_sync_ts: 0,
        }
    }
}

type LatestValuesByNetwork = HashMap<String, LatestValues>;

fn create_callback(
    latest_values: Arc<RwLock<HashMap<String, LatestValues>>>,
    value_fetcher: impl Fn(&LatestValues) -> u64,
) -> impl Fn(&dyn AsyncInstrument<u64>) {
    move |observer: &dyn AsyncInstrument<u64>| {
        let latest_values_by_network = latest_values.read().unwrap();
        for (network, latest_values) in latest_values_by_network.iter() {
            let attrs = [KeyValue::new(NETWORK, network.clone())];
            observer.observe(value_fetcher(latest_values), &attrs);
        }
    }
}

#[derive(Clone)]
pub struct RunningDefinitionsMetrics {
    latest_values_by_network: Arc<RwLock<LatestValuesByNetwork>>,
}

impl RunningDefinitionsMetrics {
    pub fn new() -> Self {
        let latest_values_by_network = Arc::new(RwLock::new(LatestValuesByNetwork::new()));
        let meter = global::meter(AXUM_APP);

        let _load_new_targets_error = meter
            .u64_observable_gauge("msd.definitions.load.errors")
            .with_callback(create_callback(latest_values_by_network.clone(), |values| values.load_new_targets_error))
            .with_description("Total number of errors while loading new targets per definition")
            .build();
        let _sync_registry_error = meter
            .u64_observable_gauge("msd.definitions.sync.errors")
            .with_callback(create_callback(latest_values_by_network.clone(), |values| values.sync_registry_error))
            .with_description("Total number of errors while syncing the registry per definition")
            .build();
        let _definitions_load_successful = meter
            .u64_observable_gauge("msd.definitions.load.successful")
            .with_callback(create_callback(latest_values_by_network.clone(), |values| {
                values.definitions_load_successful
            }))
            .with_description("Status of last load of the registry per definition")
            .build();
        let _definitions_sync_successful = meter
            .u64_observable_gauge("msd.definitions.sync.successful")
            .with_callback(create_callback(latest_values_by_network.clone(), |values| {
                values.definitions_sync_successful
            }))
            .with_description("Status of last sync of the registry with NNS of definition")
            .build();
        let _successful_sync_ts = meter
            .u64_observable_gauge("msd.definitions.sync.ts")
            .with_callback(create_callback(latest_values_by_network.clone(), |values| values.successful_sync_ts))
            .with_description("Timestamp of last successful sync")
            .build();

        Self { latest_values_by_network }
    }

    pub fn observe_sync(&self, network: String, success: bool) {
        let mut s = self.latest_values_by_network.write().unwrap();
        let latest_values = s.entry(network).or_insert(LatestValues::new());
        latest_values.definitions_sync_successful = match success {
            true => {
                let now = SystemTime::now();
                let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
                latest_values.successful_sync_ts = since_epoch.as_secs();
                1
            }
            false => {
                latest_values.sync_registry_error += 1;
                0
            }
        }
    }

    pub fn observe_load(&self, network: String, success: bool) {
        let mut s = self.latest_values_by_network.write().unwrap();
        let latest_values = s.entry(network).or_insert(LatestValues::new());
        latest_values.definitions_load_successful = match success {
            true => 1,
            false => {
                latest_values.load_new_targets_error += 1;
                0
            }
        };
    }

    pub fn observe_end(&self, network: String) {
        let mut s = self.latest_values_by_network.write().unwrap();
        s.remove(&network);
    }
}
