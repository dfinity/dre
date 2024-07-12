use candid::Principal;
use ic_cdk_macros::*;
use itertools::Itertools;
use std::time::Duration;
use types::{MetricsResponse, SubnetMetricsResponse};
mod metrics_manager;
mod stable_memory;
mod types;

const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 6;

#[init]
fn init() {
    ic_cdk_timers::set_timer_interval(Duration::from_secs(TIMER_INTERVAL_SEC), || {
        ic_cdk::spawn(async { metrics_manager::update_metrics().await.unwrap() })
    });
}

#[post_upgrade]
fn post_upgrade() {
    init();
}

#[query]
fn get_metrics(from_ts: Option<u64>) -> Vec<MetricsResponse> {
    let ts = from_ts.unwrap_or_default();

    let metrics = stable_memory::get_metrics(ts);

    metrics
        .into_iter()
        .map(|(ts, subnets_metrics)| MetricsResponse { ts, subnets_metrics })
        .collect_vec()
}

#[query]
fn get_subnet_metrics(subnet: Principal, from_ts: Option<u64>) -> Vec<SubnetMetricsResponse> {
    let ts = from_ts.unwrap_or_default();

    let metrics = stable_memory::get_metrics(ts);

    metrics
        .into_iter()
        .filter_map(|(ts, subnets_metrics)| {
            subnets_metrics
                .into_iter()
                .find(|p| p.subnet_id == subnet)
                .map(|metric| (ts, metric.node_metrics))
                .map(|(ts, node_metrics)| SubnetMetricsResponse { ts, node_metrics })
        })
        .collect_vec()
}
