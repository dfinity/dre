use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use opentelemetry::{global, metrics::Observer, KeyValue};

const NETWORK: &str = "network";
const AXUM_APP: &str = "axum-app";

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

#[derive(Clone)]
pub struct RunningDefinitionsMetrics {
    latest_values_by_network: Arc<RwLock<LatestValuesByNetwork>>,
}

impl RunningDefinitionsMetrics {
    pub fn new() -> Self {
        let latest_values_by_network = Arc::new(RwLock::new(LatestValuesByNetwork::new()));
        let meter = global::meter(AXUM_APP);
        let load_new_targets_error = meter
            .clone()
            .u64_observable_gauge("msd.definitions.load.errors")
            .with_description("Total number of errors while loading new targets per definition")
            .init();
        let sync_registry_error = meter
            .clone()
            .u64_observable_gauge("msd.definitions.sync.errors")
            .with_description("Total number of errors while syncing the registry per definition")
            .init();
        let definitions_load_successful = meter
            .clone()
            .u64_observable_gauge("msd.definitions.load.successful")
            .with_description("Status of last load of the registry per definition")
            .init();
        let definitions_sync_successful = meter
            .clone()
            .u64_observable_gauge("msd.definitions.sync.successful")
            .with_description("Status of last sync of the registry with NNS of definition")
            .init();
        let successful_sync_ts = meter
            .clone()
            .u64_observable_gauge("msd.definitions.sync.ts")
            .with_description("Timestamp of last successful sync")
            .init();
        let instruments = [
            load_new_targets_error.as_any(),
            sync_registry_error.as_any(),
            definitions_load_successful.as_any(),
            definitions_sync_successful.as_any(),
            successful_sync_ts.as_any(),
        ];
        let s = latest_values_by_network.clone();
        let update_instruments = move |observer: &dyn Observer| {
            // We blocking-lock because this is not async code, and this code
            // does not need to be async, since it just needs to read local data.
            // C.f. https://docs.rs/tokio/1.24.2/tokio/sync/struct.Mutex.html#method.blocking_lock
            let latest_values_by_network = s.read().unwrap();
            for (network, latest_values) in latest_values_by_network.iter() {
                let attrs = [KeyValue::new(NETWORK, network.clone())];
                for (instrument, measurement) in [
                    (&load_new_targets_error, latest_values.load_new_targets_error),
                    (&sync_registry_error, latest_values.sync_registry_error),
                    (&definitions_load_successful, latest_values.definitions_load_successful),
                    (&definitions_sync_successful, latest_values.definitions_sync_successful),
                    (&successful_sync_ts, latest_values.successful_sync_ts),
                ]
                .into_iter()
                {
                    observer.observe_u64(instrument, measurement, &attrs)
                }
            }
        };
        meter.register_callback(&instruments, update_instruments).unwrap();

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
