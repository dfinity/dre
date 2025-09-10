use ic_cdk::api::in_replicated_execution;
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_node_rewards_canister::canister::{current_time, NodeRewardsCanister};
use ic_node_rewards_canister::storage::{RegistryStoreStableMemoryBorrower, METRICS_MANAGER, REWARDABLE_NODES_CACHE};
use ic_node_rewards_canister::telemetry;
use ic_node_rewards_canister_api::monthly_rewards::{
    GetNodeProvidersMonthlyXdrRewardsRequest, GetNodeProvidersMonthlyXdrRewardsResponse,
};
use ic_node_rewards_canister_api::provider_rewards_calculation::{
    GetNodeProviderRewardsCalculationRequest, GetNodeProviderRewardsCalculationResponse,
};
use ic_node_rewards_canister_api::providers_rewards::{
    GetNodeProvidersRewardsRequest, GetNodeProvidersRewardsResponse,
};
use ic_registry_canister_client::StableCanisterRegistryClient;
use rewards_calculation::types::DayUtc;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use itertools::Itertools;

fn main() {}

thread_local! {
    static REGISTRY_STORE: Arc<StableCanisterRegistryClient<RegistryStoreStableMemoryBorrower>> = {
        let store = StableCanisterRegistryClient::<RegistryStoreStableMemoryBorrower>::new(
            Arc::new(RegistryCanister::new()));
        Arc::new(store)
    };
    static CANISTER: RefCell<NodeRewardsCanister> = {
        let registry_store = REGISTRY_STORE.with(|store| {
            store.clone()
        });
        let metrics_manager = METRICS_MANAGER.with(|m| m.clone());

        RefCell::new(NodeRewardsCanister::new(registry_store, metrics_manager))
    };
}

#[init]
fn canister_init() {
    schedule_timers();
}

#[pre_upgrade]
fn pre_upgrade() {}

#[post_upgrade]
fn post_upgrade() {
    schedule_timers();
}

// The frequency of regular registry syncs.  This is set to 1 hour to avoid
// making too many requests.  Before meaningful calculations are made, however, the
// registry data should be updated.
const SYNC_INTERVAL_SECONDS: Duration = Duration::from_secs(60 * 60); // 1 hour
const DAY_IN_SECONDS: u64 = 60 * 60 * 24;
const SYNC_AT_SECONDS_AFTER_MIDNIGHT: u64 = 10;
const MAX_SYNC_DURATION_SECONDS: u64 = 10 * 60;
const MAX_REWARDABLE_NODES_BACKFILL_DAYS: u64 = 100;
const REWARDABLE_NODES_BACKFILL_DAYS_STEP: usize = 10;

fn schedule_timers() {
    let now_secs = current_time().as_secs_since_unix_epoch();
    let since_midnight = now_secs % DAY_IN_SECONDS;
    let mut next_sync_target = now_secs - since_midnight + SYNC_AT_SECONDS_AFTER_MIDNIGHT;
    if since_midnight > SYNC_AT_SECONDS_AFTER_MIDNIGHT {
        // already past today's SYNC_AT_SECONDS_AFTER_MIDNIGHT → use tomorrow
        next_sync_target = next_sync_target + DAY_IN_SECONDS;
    };

    ic_cdk_timers::set_timer(Duration::from_secs(0), || {
        schedule_daily_sync()
    });

    ic_cdk_timers::set_timer(Duration::from_secs(20), || {
        let count = REWARDABLE_NODES_CACHE.with_borrow(|cache| {
            cache.iter()
                .map(|(k, _)| k.registry_version)
                .unique()
                .count()
        });
        ic_cdk::println!("Rewardable nodes cache size: {}", count);
    });
}

fn schedule_daily_sync() {
    ic_cdk::futures::spawn_017_compat(async move {
        let counter = telemetry::InstructionCounter::default();
        let registry_sync_result = NodeRewardsCanister::schedule_registry_sync(&CANISTER).await;
        match registry_sync_result {
            Ok(_) => {
                NodeRewardsCanister::schedule_metrics_sync(&CANISTER).await;
                backfill_rewardable_nodes_in_batches();
            }
            Err(e) => {
                ic_cdk::println!("Failed to sync local registry: {:?}", e)
            }
        }
        ic_cdk::println!("schedule_daily_sync: {:?}", counter.sum());
    });
}

fn backfill_rewardable_nodes_in_batches() {
    let now = current_time();
    let start_backfill = now.saturating_sub(Duration::from_secs(
        MAX_REWARDABLE_NODES_BACKFILL_DAYS * DAY_IN_SECONDS,
    ));

    let today: DayUtc = now.as_nanos_since_unix_epoch().into();
    let yesterday: DayUtc = today.days_until(&today).unwrap().first().unwrap().clone();
    let start_backfill_day: DayUtc = start_backfill.as_nanos_since_unix_epoch().into();
    let backfill_days: Vec<DayUtc> = start_backfill_day.days_until(&yesterday).unwrap();

    for batch in backfill_days.chunks(REWARDABLE_NODES_BACKFILL_DAYS_STEP) {
        let batch = batch.to_vec();
        ic_cdk_timers::set_timer(Duration::from_secs(0), move || {
            let counter = telemetry::InstructionCounter::default();
            for day in batch {
                NodeRewardsCanister::backfill_rewardable_nodes(&CANISTER, &day)
                    .unwrap_or_else(|e| ic_cdk::println!("Failed to backfill: {:?}", e));
            }
            ic_cdk::println!("backfill_rewardable_nodes_in_batches: {:?}", counter.sum());
        });
    }
}

#[cfg(any(feature = "test", test))]
#[query(hidden = true)]
fn get_registry_value(key: String) -> Result<Option<Vec<u8>>, String> {
    CANISTER.with(|canister| canister.borrow().get_registry_value(key))
}

#[update]
async fn get_node_providers_monthly_xdr_rewards(
    request: GetNodeProvidersMonthlyXdrRewardsRequest,
) -> GetNodeProvidersMonthlyXdrRewardsResponse {
    NodeRewardsCanister::get_node_providers_monthly_xdr_rewards(&CANISTER, request).await
}

#[update]
async fn get_node_providers_rewards(
    request: GetNodeProvidersRewardsRequest,
) -> GetNodeProvidersRewardsResponse {
    NodeRewardsCanister::get_node_providers_rewards(&CANISTER, request)
}

#[update]
fn get_node_provider_rewards_calculation(
    request: GetNodeProviderRewardsCalculationRequest,
) -> GetNodeProviderRewardsCalculationResponse {
    let mut instruction_counter = telemetry::InstructionCounter::default();
    let res = NodeRewardsCanister::get_node_provider_rewards_calculation(&CANISTER, request);
    ic_cdk::println!("get_node_provider_rewards_calculation: {:?}", instruction_counter.sum());
    res
}
