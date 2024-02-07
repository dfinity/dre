use opentelemetry::{global, metrics::UpDownCounter};

#[derive(Clone)]
pub struct MSDMetrics {
    pub definitions: UpDownCounter<i64>,
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
            .i64_up_down_counter("msd.definitions")
            .with_description("Total number of definitions that multiservice discovery is scraping")
            .init();

        Self { definitions }
    }
}
