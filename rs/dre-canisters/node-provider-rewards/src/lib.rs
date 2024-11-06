use chrono_utils::DateTimeRange;
use ic_cdk_macros::*;
use local_registry::LocalRegistry;
use registry_querier::RegistryQuerier;
use rewards_manager::RewardsManager;
use std::{cell::RefCell, rc::Rc};
use types::NodeProviderXDRRewardsArgs;
mod chrono_utils;
mod computation_logger;
mod metrics;
mod local_registry;
mod registry_querier;
mod rewards_manager;
mod stable_memory;
mod types;

// Management canisters updates node metrics every day
const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 24;

thread_local! {
    static LOCAL_REGISTRY: RefCell<Rc<LocalRegistry>> = RefCell::new(Rc::new(Default::default()));
}

async fn sync_node_metrics_task() {
    match metrics::sync_node_metrics().await {
        Ok(_) => {
            ic_cdk::println!("Successfully updated trustworthy node metrics");
        }
        Err(e) => {
            ic_cdk::println!("Error syncing metrics: {}", e);
        }
    }
}

async fn update_local_registry() {
    // sync_node_metrics_task().await;
    let local_registry = LOCAL_REGISTRY.with_borrow(|local_registry| local_registry.clone());
    match local_registry.sync_registry_stored().await {
         Ok(_) => {
             ic_cdk::println!("Successfully sync_registry_stored");
         }
         Err(e) => {
             ic_cdk::println!("Error sync_registry_stored: {}", e);
         }
     }

    get_node_providers_xdr_rewards(NodeProviderXDRRewardsArgs{from_ts: 1730187565000000000, to_ts: 1730894985000000000}).await;
}

fn setup_timers() {
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(0), || ic_cdk::spawn(update_local_registry()));
}

#[init]
fn init() {
    setup_timers();
}

#[post_upgrade]
fn post_upgrade() {
    setup_timers();
}

#[update]
async fn get_node_providers_xdr_rewards(args: NodeProviderXDRRewardsArgs) {
    let rewarding_period: DateTimeRange = DateTimeRange::new(args.from_ts, args.to_ts);
    ic_cdk::println!("Computing and storing rewards for {}", rewarding_period);

    let registry_querier = RegistryQuerier {
        local_registry: LOCAL_REGISTRY.with_borrow(|local_registry| local_registry.clone()),
    };
    let rewards_manager = RewardsManager::new(registry_querier, rewarding_period);

    rewards_manager.compute_node_providers_rewards().await;
}
