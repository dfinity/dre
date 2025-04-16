use std::cell::RefCell;

#[derive(Default)]
pub struct PrometheusMetrics {
    /// Records the time the last sync began.
    pub last_sync_start: f64,
    /// Records the time that sync last succeeded.
    pub last_sync_success: f64,
    /// Records the time that sync last ended (successfully or in failure).
    /// If last_sync_start > last_sync_end, sync is in progress, else sync is not taking place.
    /// If last_sync_success == last_sync_end, last sync was successful.
    pub last_sync_end: f64,
}

impl PrometheusMetrics {
    fn new() -> Self {
        Default::default()
    }

    pub fn mark_last_calculation_start(&mut self) {
        self.last_sync_start = (ic_cdk::api::time() / 1_000_000_000) as f64
    }

    pub fn mark_last_calculation_success(&mut self) {
        self.last_sync_end = (ic_cdk::api::time() / 1_000_000_000) as f64;
        self.last_sync_success = self.last_sync_end
    }

    pub fn mark_last_calculation_end(&mut self) {
        self.last_sync_end = (ic_cdk::api::time() / 1_000_000_000) as f64
    }
}

thread_local! {
    pub(crate) static PROMETHEUS_METRICS: RefCell<PrometheusMetrics> = RefCell::new(PrometheusMetrics::new());
}
