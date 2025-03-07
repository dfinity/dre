use crate::storage::{MetricsManagerInstance, RegistryStoreInstance};
use ic_cdk_macros::*;

mod metrics;
mod metrics_types;
mod registry;
mod registry_store;
mod registry_store_types;
mod storage;

// Management canisters updates node metrics every day
const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;

async fn sync_all() {
    RegistryStoreInstance::sync_registry_stored()
        .await
        .expect("Failed to sync registry stored");

    let subnets_list = registry::subnets_list();
    MetricsManagerInstance::sync_subnets_metrics(subnets_list).await;

    ic_cdk::println!("Successfully synced subnets metrics and local registry");
}

async fn retry_metrics_fetching() {
    MetricsManagerInstance::retry_metrics_fetching().await;
}

fn setup_timers() {
    // At 1 AM UTC every day sync metrics and registry
    let utc_1_am = DAY_IN_SECONDS + HOUR_IN_SECONDS - (ic_cdk::api::time() / 1_000_000_000) % DAY_IN_SECONDS;

    ic_cdk_timers::set_timer(std::time::Duration::from_secs(utc_1_am), || {
        ic_cdk::spawn(sync_all());

        // Reschedule for the next day
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(DAY_IN_SECONDS), || ic_cdk::spawn(sync_all()));
    });

    // Retry subnets fetching every hour
    ic_cdk_timers::set_timer_interval(
        std::time::Duration::from_secs(HOUR_IN_SECONDS),
        || ic_cdk::spawn(retry_metrics_fetching()),
    );
}

#[init]
fn init() {
    setup_timers();
}

#[pre_upgrade]
fn pre_upgrade() {
    storage::pre_upgrade()
}

#[post_upgrade]
fn post_upgrade() {
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(0), || ic_cdk::spawn(sync_all()));
    setup_timers();
}
