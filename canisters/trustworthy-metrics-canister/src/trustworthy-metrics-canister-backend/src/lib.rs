use std::time::Duration;
use ic_cdk_macros::*;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use node_metrics_manager::NodeMetricsManager;
pub mod node_metrics_manager;

const TIMER_INTERVAL_SEC: u64 = 600000;
#[init]
fn init() {
    ic_cdk_timers::set_timer_interval(Duration::from_secs(TIMER_INTERVAL_SEC), || {
        ic_cdk::spawn(async {
            let manager = NodeMetricsManager::new();
            manager.refresh().await.unwrap()
        })
    });
}

#[post_upgrade]
fn post_upgrade() {
    init();
}

#[query]
fn subnet_node_metrics_history() -> Result<NodeMetricsHistoryResponse, String>  {
    ic_cdk::print("init");
    Result::Err("todo".to_string())
}
