use std::{collections::HashMap, sync::Arc};

use opentelemetry::{
    global,
    metrics::{CallbackRegistration, Counter, ObservableGauge},
    KeyValue,
};
use slog::{error, info, Logger};
use tokio::sync::Mutex;

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

#[derive(Clone)]
pub struct RunningDefinitionsMetrics {
    pub load_new_targets_error: Counter<u64>,
    pub definitions_load_successful: ObservableGauge<i64>,

    pub sync_registry_error: Counter<u64>,
    pub definitions_sync_successful: ObservableGauge<i64>,

    pub definition_callbacks: Arc<Mutex<HashMap<String, Vec<Box<dyn CallbackRegistration>>>>>,
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
            definition_callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn set_successful_sync(&mut self, network: String, logger: Logger) {
        Self::set_callback(
            network,
            logger,
            1,
            &self.definitions_sync_successful,
            &self.definition_callbacks,
        )
        .await
    }

    pub async fn set_failed_sync(&mut self, network: String, logger: Logger) {
        Self::set_callback(
            network,
            logger,
            0,
            &self.definitions_sync_successful,
            &self.definition_callbacks,
        )
        .await
    }

    pub async fn set_successful_load(&mut self, network: String, logger: Logger) {
        Self::set_callback(
            network,
            logger,
            1,
            &self.definitions_load_successful,
            &self.definition_callbacks,
        )
        .await
    }

    pub async fn set_failed_load(&mut self, network: String, logger: Logger) {
        Self::set_callback(
            network,
            logger,
            0,
            &self.definitions_load_successful,
            &self.definition_callbacks,
        )
        .await
    }

    async fn set_callback(
        network: String,
        logger: Logger,
        status: i64,
        gague: &ObservableGauge<i64>,
        callbacks: &Arc<Mutex<HashMap<String, Vec<Box<dyn CallbackRegistration>>>>>,
    ) {
        let meter = global::meter(AXUM_APP);
        let network_clone = network.clone();
        let local_clone = gague.clone();

        match meter.register_callback(&[local_clone.as_any()], move |observer| {
            observer.observe_i64(&local_clone, status, &[KeyValue::new(NETWORK, network.clone())])
        }) {
            Ok(callback) => {
                info!(logger, "Registering callback for '{}'", &network_clone);
                let mut locked = callbacks.lock().await;

                if let Some(definition) = locked.get_mut(&network_clone) {
                    definition.push(callback)
                } else {
                    locked.insert(network_clone, vec![callback]);
                }
            }
            Err(e) => error!(
                logger,
                "Couldn't register callback for network '{}': {:?}", network_clone, e
            ),
        }
    }

    pub async fn unregister_callback(&self, network: String, logger: Logger) {
        let mut locked = self.definition_callbacks.lock().await;

        if let Some(callbacks) = locked.remove(&network) {
            for mut callback in callbacks {
                if let Err(e) = callback.unregister() {
                    error!(
                        logger,
                        "Couldn't unregister callback for network '{}': {:?}", network, e
                    )
                }
            }
        } else {
            error!(
                logger,
                "Couldn't unregister callbacks for network '{}': key not found", &network
            )
        }
    }
}
