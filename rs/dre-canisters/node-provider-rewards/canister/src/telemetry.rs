use std::cell::RefCell;

#[derive(Default)]
pub struct PrometheusMetrics {
    last_calculation_start: f64,
    last_calculation_success: f64,
    last_calculation_end: f64,
}

impl PrometheusMetrics {
    fn new() -> Self {
        Default::default()
    }

    pub fn mark_last_calculation_start(&mut self) {
        self.last_calculation_start = (ic_cdk::api::time() / 1_000_000_000) as f64
    }

    pub fn mark_last_calculation_success(&mut self) {
        self.last_calculation_end = (ic_cdk::api::time() / 1_000_000_000) as f64;
        self.last_calculation_success = self.last_calculation_end
    }

    pub fn mark_last_calculation_end(&mut self) {
        self.last_calculation_end = (ic_cdk::api::time() / 1_000_000_000) as f64
    }
}

thread_local! {
    pub(crate) static PROMETHEUS_METRICS: RefCell<PrometheusMetrics> = RefCell::new(PrometheusMetrics::new());
}
