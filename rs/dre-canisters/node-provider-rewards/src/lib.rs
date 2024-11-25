use crate::canister_data_provider::StableMemoryStore;
use crate::data_provider::CanisterDataProvider;
use chrono_utils::DateTimeRange;
use ic_cdk_macros::*;
use ic_registry_canister_client::CanisterRegistryClient;
use std::cell::RefCell;
use std::sync::Arc;
use types::NodeProviderXDRRewardsArgs;

mod canister_data_provider;
mod chrono_utils;
mod computation_logger;
mod data_provider;
mod metrics;
mod stable_memory;
mod types;

// Management canisters updates node metrics every day
const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 24;

thread_local! {
    static DATA_PROVIDER: RefCell<Arc<CanisterDataProvider<StableMemoryStore>>> =
        RefCell::new(Arc::new(
            CanisterDataProvider::new(Default::default())
    ));
    static REGISTRY_CLIENT: RefCell<CanisterRegistryClient> =
        RefCell::new(CanisterRegistryClient::new(DATA_PROVIDER.with_borrow(|store| store.clone())));
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
    // update store

    stable_memory::REGISTRY.with_borrow_mut(|registry_stored| registry_stored.clear_new());
    let local_registry = DATA_PROVIDER.with_borrow(|provider| provider.clone());
    match local_registry.sync_registry_stored().await {
        Ok(_) => {
            ic_cdk::println!("Successfully sync_registry_stored");
        }
        Err(e) => {
            ic_cdk::println!("Error sync_registry_stored: {}", e);
        }
    }

    // update cache

    REGISTRY_CLIENT.with_borrow(|registry_client| registry_client.update_to_latest_version());
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
}
