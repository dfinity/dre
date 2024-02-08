use opentelemetry::{
    global,
    metrics::{Counter, UpDownCounter},
};

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
        let meter = global::meter("axum-app");
        let definitions = meter
            .i64_up_down_counter("msd.definitions.total")
            .with_description("Total number of definitions that multiservice discovery is scraping")
            .init();

        Self {
            definitions,
            running_definition_metrics: RunningDefinitionsMetrics::new(),
        }
    }
}

#[derive(Clone)]
pub struct RunningDefinitionsMetrics {
    pub load_new_targets_error: Counter<u64>,
    pub sync_registry_error: Counter<u64>,
}

impl RunningDefinitionsMetrics {
    pub fn new() -> Self {
        let meter = global::meter("axum-app");
        let load_new_targets_error = meter
            .u64_counter("msd.definitions.load.errors")
            .with_description("Total number of errors while loading new targets per definition")
            .init();

        let sync_registry_error = meter
            .u64_counter("msd.definitions.sync.errors")
            .with_description("Total number of errors while syncing the registry per definition")
            .init();

        Self {
            load_new_targets_error,
            sync_registry_error,
        }
    }
}
