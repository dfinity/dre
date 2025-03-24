use crate::canister_client::ICCanisterClient;
use crate::registry_store::CanisterRegistryStore;
use crate::storage::{State, METRICS_MANAGER, VM};
use ic_canisters_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_cdk_macros::*;
use ic_nervous_system_common::serve_metrics;
use std::cell::RefCell;

mod canister_client;
mod metrics;
mod metrics_types;
mod registry;
mod registry_store;
mod registry_store_types;
mod storage;

const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;

pub type RegistryStoreInstance = CanisterRegistryStore<State, VM>;
pub const IC_CANISTER_CLIENT: ICCanisterClient = ICCanisterClient;

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

    fn mark_last_calculation_start(&mut self) {
        self.last_calculation_start = (ic_cdk::api::time() / 1_000_000_000) as f64
    }

    fn mark_last_calculation_success(&mut self) {
        self.last_calculation_end = (ic_cdk::api::time() / 1_000_000_000) as f64;
        self.last_calculation_success = self.last_calculation_end
    }

    fn mark_last_calculation_end(&mut self) {
        self.last_calculation_end = (ic_cdk::api::time() / 1_000_000_000) as f64
    }
}

thread_local! {
    pub(crate) static PROMETHEUS_METRICS: RefCell<PrometheusMetrics> = RefCell::new(PrometheusMetrics::new());
}

/// Sync the local registry and subnets metrics with remote
///
/// - Sync local registry stored from the remote registry canister
/// - Sync subnets metrics from the management canister of the different subnets
async fn sync_all() {
    PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_calculation_start());
    let registry_sync_result = RegistryStoreInstance::sync_registry_stored(&IC_CANISTER_CLIENT).await;

    match registry_sync_result {
        Ok(_) => {
            let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
            let subnets_list = registry::subnets_list();

            metrics_manager.update_subnets_metrics(subnets_list).await;
            PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_calculation_success());
            ic_cdk::println!("Successfully synced subnets metrics and local registry");
        }
        Err(e) => {
            PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_calculation_end());
            ic_cdk::println!("Failed to sync local registry: {:?}", e)
        }
    }
}

fn setup_timers() {
    // Next 1 AM UTC timestamp
    let next_utc_1am_sec = DAY_IN_SECONDS + HOUR_IN_SECONDS - (ic_cdk::api::time() / 1_000_000_000) % DAY_IN_SECONDS;
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(next_utc_1am_sec), || {
        ic_cdk::spawn(sync_all());

        // Reschedule for the next day
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(DAY_IN_SECONDS), || ic_cdk::spawn(sync_all()));
    });

    // Retry subnets fetching every hour
    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HOUR_IN_SECONDS), || {
        ic_cdk::spawn(async {
            let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
            metrics_manager.retry_failed_subnets().await;
        });
    });
}

#[init]
fn init() {
    setup_timers();
}

#[post_upgrade]
fn post_upgrade() {
    setup_timers();
}

pub fn encode_metrics(metrics: &PrometheusMetrics, w: &mut ic_metrics_encoder::MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    // General resource consumption.
    w.encode_gauge(
        "canister_stable_memory_size_bytes",
        ic_nervous_system_common::stable_memory_size_bytes() as f64,
        "Size of the stable memory allocated by this canister measured in bytes.",
    )?;
    w.encode_gauge(
        "canister_total_memory_size_bytes",
        ic_nervous_system_common::total_memory_size_bytes() as f64,
        "Size of the total memory allocated by this canister measured in bytes.",
    )?;

    // Calculation signals.

    // Calculation start timestamp seconds.
    //
    // * 0.0 -> first calculation not yet begun since canister started.
    // * Any other positive number -> at least one calculation has started.
    w.encode_gauge(
        "last_calculation_start_timestamp_seconds",
        metrics.last_calculation_start,
        "Last time the calculation of metrics started.  If this metric is present but zero, the first calculation during this canister's current execution has not yet begun or taken place.",
    )?;
    // Calculation finish timestamp seconds.
    // * 0.0 -> first calculation not yet finished since canister started.
    // * last_calculation_end_timestamp_seconds - last_calculation_start_timestamp_seconds > 0 -> last calculation finished, next calculation not started yet
    // * last_calculation_end_timestamp_seconds - last_calculation_start_timestamp_seconds < 0 -> calculation ongoing, not finished yet
    w.encode_gauge(
        "last_calculation_end_timestamp_seconds",
        metrics.last_calculation_end,
        "Last time the calculation of metrics ended (successfully or with failure).  If this metric is present but zero, the first calculation during this canister's current execution has not started or finished yet, either successfully or with errors.   Else, subtracting this from the last calculation start should yield a positive value if the calculation ended (successfully or with errors), and a negative value if the calculation is still ongoing but has not finished.",
    )?;
    // Calculation success timestamp seconds.
    // * 0.0 -> no calculation has yet succeeded since canister started.
    // * last_calculation_end_timestamp_seconds == last_calculation_success_timestamp_seconds -> last calculation finished successfully
    // * last_calculation_end_timestamp_seconds != last_calculation_success_timestamp_seconds -> last calculation failed
    w.encode_gauge(
        "last_calculation_success_timestamp_seconds",
        metrics.last_calculation_success,
        "Last time the calculation of metrics succeeded.  If this metric is present but zero, no calculation has yet succeeded during this canister's current execution.  Else, subtracting this number from last_calculation_start_timestamp_seconds gives a positive time delta when the last calculation succeeded, or a negative value if either the last calculation failed or a calculation is currently being performed.  By definition, this and last_calculation_end_timestamp_seconds will be identical when the last calculation succeeded.",
    )?;

    Ok(())
}

#[query(hidden = true, decoding_quota = 10000)]
fn http_request(request: HttpRequest) -> HttpResponse {
    match request.path() {
        "/metrics" => serve_metrics(|encoder| PROMETHEUS_METRICS.with(|m| encode_metrics(&m.borrow(), encoder))),
        _ => HttpResponseBuilder::not_found().build(),
    }
}
