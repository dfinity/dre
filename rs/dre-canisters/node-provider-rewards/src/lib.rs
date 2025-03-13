use crate::canister_client::ICCanisterClient;
use crate::metrics::MetricsManager;
use crate::registry_store::CanisterRegistryStore;
use crate::storage::{State, VM};
use ic_cdk_macros::*;

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
pub type MetricsManagerInstance = MetricsManager<State, VM>;
pub const IC_CANISTER_CLIENT: ICCanisterClient = ICCanisterClient {};

/// Sync the local registry and subnets metrics with remote
///
/// - Sync local registry stored from the remote registry canister
/// - Sync subnets metrics from the management canister of the different subnets
async fn sync_all() {
    let registry_sync_result = RegistryStoreInstance::sync_registry_stored(&IC_CANISTER_CLIENT).await;

    match registry_sync_result {
        Ok(_) => {
            let subnets_list = registry::subnets_list();
            MetricsManagerInstance::update_subnets_metrics(&IC_CANISTER_CLIENT, subnets_list).await;

            ic_cdk::println!("Successfully synced subnets metrics and local registry");
        }
        Err(e) => ic_cdk::println!("Failed to sync local registry: {:?}", e),
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
        ic_cdk::spawn(MetricsManagerInstance::retry_failed_subnets(&IC_CANISTER_CLIENT));
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
