use candid::Principal;
use chrono_utils::DateTimeRange;
use ic_cdk_macros::*;
use itertools::Itertools;
use registry::LocalRegistry;
use rewards_manager::RewardsManager;
use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, collections::{btree_map::Entry, BTreeMap}, thread::sleep};
use types::SubnetNodeMetricsArgs;
mod chrono_utils;
mod computation_logger;
mod nodes_manager;
mod rewards_manager;
mod stable_memory;
mod types;
mod registry;
mod fake_registry_client;

// Management canisters updates node metrics every day
const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 24;

thread_local! {
    static LOCAL_REGISTRY: RefCell<Option<LocalRegistry>> = RefCell::new(Some(LocalRegistry::new()));
}

async fn sync_node_metrics_task() {
    match nodes_manager::sync_node_metrics().await {
        Ok(_) => {
            ic_cdk::println!("Successfully updated trustworthy node metrics");
        }
        Err(e) => {
            ic_cdk::println!("Error syncing metrics: {}", e);
        }
    }
}

async fn update_local_registry() {
    let local_registry = LOCAL_REGISTRY.borrow().take();

    if let Some(local_registry) = local_registry {
        match local_registry.sync_registry_stored().await {
            Ok(_) => {
                ic_cdk::println!("Successfully sync_registry_stored");
            }
            Err(e) => {
                ic_cdk::println!("Error sync_registry_stored: {}", e);
            }
        }
        LOCAL_REGISTRY.with_borrow_mut(|registry| *registry = Some(local_registry));
    }
}

fn setup_timers(){
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
async fn get_node_providers_monthly_xdr_rewards(args: SubnetNodeMetricsArgs) {
    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(TIMER_INTERVAL_SEC), || ic_cdk::spawn(sync_node_metrics_task()));
    let mut rewards_manager = RewardsManager::new();
    let now = ic_cdk::api::time();
    let rewarding_period: DateTimeRange = DateTimeRange::last_reward_period(now);

    ic_cdk::println!("Computing and storing rewards for {}", rewarding_period);
    let node_operators_rewardables = nodes_manager::get_node_operators_rewardables(&rewarding_period).await.unwrap();

    let rewards_table = nodes_manager::get_rewards_table().await.unwrap();
    let assigned_nodes_performance = nodes_manager::get_assigned_nodes_performance(node_operators_rewardables.clone(), &rewarding_period).await.unwrap();
    let node_providers_rewardables = nodes_manager::get_node_providers_rewardables(node_operators_rewardables);

    rewards_manager.compute_node_providers_rewards(node_providers_rewardables, assigned_nodes_performance, rewarding_period, rewards_table);
}

