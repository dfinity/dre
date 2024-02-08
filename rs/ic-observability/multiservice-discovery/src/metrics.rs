use opentelemetry::{global, metrics::ObservableGauge};

#[derive(Clone)]
pub struct MSDMetrics {
    pub definitions: ObservableGauge<u64>,
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
            .u64_observable_gauge("msd.definitions")
            .with_description("Total number of definitions that multiservice discovery is scraping")
            .init();

        Self { definitions }
    }
}
