use ic_cdk_macros::*;
use itertools::Itertools;
use std::time::Duration;
use types::{SubnetNodeMetricsArgs, SubnetNodeMetricsResponse};
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
fn subnet_node_metrics(args: SubnetNodeMetricsArgs) -> Result<Vec<SubnetNodeMetricsResponse>, String> {
    let from_ts = args.ts.unwrap_or_default();

    let metrics = stable_memory::get_metrics(from_ts);

    let metrics_flat = metrics
        .into_iter()
        .flat_map(|(ts, subnets)| subnets
            .into_iter()
            .map(move |subnet_node_metrics| SubnetNodeMetricsResponse{
                ts, 
                subnet_id: subnet_node_metrics.subnet_id, 
                node_metrics: subnet_node_metrics.node_metrics
            })
        )
        .collect_vec();

    let result = match args.subnet_id {
        Some(subnet_id) => {
            metrics_flat
                .into_iter()
                .filter(|metrics| metrics.subnet_id == subnet_id)
                .collect_vec()
        },
        None => metrics_flat
    };

    Ok(result)
}
