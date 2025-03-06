use crate::storage::{with_metrics_manager, with_registry_client, with_registry_store};
use ic_base_types::SubnetId;
use ic_cdk_macros::*;
use ic_stable_structures::memory_manager::MemoryId;
use std::future::Future;
use std::ops::DerefMut;
use std::sync::Arc;

mod metrics;
mod registry;
mod registry_store;
mod storage;

// Management canisters updates node metrics every day
const HR_IN_SEC: u64 = 60 * 60;
const DAY_SECONDS: u64 = HR_IN_SEC * 24;

async fn sync_all() {
    with_registry_store(|registry_store| {
        let cloned = Arc::clone(&registry_store);
        async move {
            let mut store = cloned.write().unwrap();
            store.sync_registry_stored().await.expect("Failed to sync registry stored");
        }
    })
    .await;

    let subnets_list = with_registry_client(|registry_client| registry_client.subnets_list());
    with_metrics_manager(|metrics_manager| {
        let cloned = Arc::clone(&metrics_manager);
        async move {
            let mut metrics_manager = cloned.write().unwrap();
            metrics_manager.sync_subnets_metrics(&subnets_list).await;
        }
    })
    .await;
}

fn setup_timers() {
    // At 1 AM UTC every day sync metrics and registry
    let utc_1_am = DAY_SECONDS + HR_IN_SEC - (ic_cdk::api::time() / 1_000_000_000) % DAY_SECONDS;

    ic_cdk_timers::set_timer(std::time::Duration::from_secs(utc_1_am), || {
        ic_cdk::spawn(sync_all());
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(DAY_SECONDS), || ic_cdk::spawn(sync_all()));
    });

    // Retry subnets fetching every hour
    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HR_IN_SEC), || {
        let subnets_to_retry: Vec<SubnetId> =
            with_metrics_manager(|metrics_manager| metrics_manager.read().expect("").subnets_to_retry.clone().into_iter().collect());

        if !subnets_to_retry.is_empty() {
            ic_cdk::spawn(with_metrics_manager(|metrics_manager| {
                let cloned = Arc::clone(&metrics_manager);
                async move {
                    let mut metrics_manager = cloned.write().unwrap();
                    metrics_manager.sync_subnets_metrics(&subnets_to_retry).await;
                }
            }));
        } else {
            ic_cdk::println!("No subnets to retry");
        }
    });
}

#[init]
fn init() {
    setup_timers();
}

#[pre_upgrade]
fn pre_upgrade() {}

#[post_upgrade]
fn post_upgrade() {
    setup_timers();
}
