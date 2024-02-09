use opentelemetry::{
    global,
    metrics::{Counter, ObservableGauge, UpDownCounter},
    KeyValue,
};
use slog::{error, Logger};

const NETWORK: &str = "network";
const AXUM_APP: &str = "axum-app";

#[derive(Clone)]
pub struct MSDMetrics {
    pub definitions: UpDownCounter<i64>,
    pub running_definition_metrics: RunningDefinitionsMetrics,
}

impl Default for MSDMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MSDMetrics {
    pub fn new() -> Self {
        let meter = global::meter(AXUM_APP);
        let definitions = meter
            .i64_up_down_counter("msd.definitions")
            .with_description("Total number of definitions that multiservice discovery is scraping")
            .init();

        Self {
            definitions,
            running_definition_metrics: RunningDefinitionsMetrics::new(),
        }
    }

    pub fn inc(&self, network: String) {
        self.definitions.add(1, &[KeyValue::new(NETWORK, network)])
    }

    pub fn dec(&self, network: String) {
        self.definitions.add(-1, &[KeyValue::new(NETWORK, network)])
    }
}

#[derive(Clone)]
pub struct RunningDefinitionsMetrics {
    pub load_new_targets_error: Counter<u64>,
    pub definitions_load_successful: ObservableGauge<i64>,

    pub sync_registry_error: Counter<u64>,
    pub definitions_sync_successful: ObservableGauge<i64>,
}

impl RunningDefinitionsMetrics {
    pub fn new() -> Self {
        let meter = global::meter(AXUM_APP);
        let load_new_targets_error = meter
            .u64_counter("msd.definitions.load.errors")
            .with_description("Total number of errors while loading new targets per definition")
            .init();

        let sync_registry_error = meter
            .u64_counter("msd.definitions.sync.errors")
            .with_description("Total number of errors while syncing the registry per definition")
            .init();

        let definitions_load_successful = meter
            .i64_observable_gauge("msd.definitions.load.successful")
            .with_description("Status of last load of the registry per definition")
            .init();

        let definitions_sync_successful = meter
            .i64_observable_gauge("msd.definitions.sync.successful")
            .with_description("Status of last sync of the registry with NNS of definition")
            .init();

        Self {
            load_new_targets_error,
            definitions_load_successful,
            sync_registry_error,
            definitions_sync_successful,
        }
    }

    pub fn set_successful_sync(&self, network: String, logger: Logger) {
        self.set_sync_callback(network, logger, 1)
    }

    pub fn set_failed_sync(&mut self, network: String, logger: Logger) {
        self.set_sync_callback(network, logger, 0)
    }

    fn set_sync_callback(&self, network: String, logger: Logger, status: i64) {
        let meter = global::meter(AXUM_APP);
        let network_clone = network.clone();
        let local_clone = self.definitions_sync_successful.clone();

        if let Err(e) = meter.register_callback(&[local_clone.as_any()], move |observer| {
            observer.observe_i64(&local_clone, status, &[KeyValue::new(NETWORK, network.clone())])
        }) {
            error!(
                logger,
                "Couldn't register callback for network '{}': {:?}", network_clone, e
            )
        };
    }

    pub fn set_successful_load(&mut self, network: String, logger: Logger) {
        self.set_load_callback(network, logger, 1)
    }

    pub fn set_failed_load(&mut self, network: String, logger: Logger) {
        self.set_load_callback(network, logger, 0)
    }

    fn set_load_callback(&self, network: String, logger: Logger, status: i64) {
        let meter = global::meter(AXUM_APP);
        let network_clone = network.clone();
        let local_clone = self.definitions_load_successful.clone();

        if let Err(e) = meter.register_callback(&[local_clone.as_any()], move |observer| {
            observer.observe_i64(&local_clone, status, &[KeyValue::new(NETWORK, network.clone())])
        }) {
            error!(
                logger,
                "Couldn't register callback for network '{}': {:?}", network_clone, e
            )
        };
    }
}
