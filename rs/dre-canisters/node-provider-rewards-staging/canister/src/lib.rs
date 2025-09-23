use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_node_rewards_canister::canister::NodeRewardsCanister;
use ic_node_rewards_canister::storage::{METRICS_MANAGER, RegistryStoreStableMemoryBorrower};
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
use itertools::Itertools;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use candid::Encode;

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

fn schedule_timers() {
    ic_cdk_timers::set_timer_interval(SYNC_INTERVAL_SECONDS, move || {
        ic_cdk::futures::spawn_017_compat(async move {
            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_start());
            let mut instruction_counter = telemetry::InstructionCounter::default();
            instruction_counter.lap();
            let registry_sync_result = NodeRewardsCanister::schedule_registry_sync(&CANISTER).await;
            let registry_sync_instructions = instruction_counter.lap();

            let mut metrics_sync_instructions: u64 = 0;
            match registry_sync_result {
                Ok(_) => {
                    instruction_counter.lap();
                    NodeRewardsCanister::schedule_metrics_sync(&CANISTER).await;
                    metrics_sync_instructions = instruction_counter.lap();
                    ic_cdk::println!("Successfully synced subnets metrics and local registry");
                }
                Err(e) => {
                    ic_cdk::println!("Failed to sync local registry: {:?}", e)
                }
            }

            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| {
                m.record_last_sync_instructions(
                    instruction_counter.sum(),
                    registry_sync_instructions,
                    metrics_sync_instructions,
                )
            });
        });
    });
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
    let instructions = telemetry::InstructionCounter::default();
    let res = NodeRewardsCanister::get_node_providers_rewards(&CANISTER, request).await;
    ic_cdk::println!("get_node_providers_rewards instructions: {:?}", instructions.sum());
    res
}

#[query]
fn get_node_provider_rewards_calculation(
    request: GetNodeProviderRewardsCalculationRequest,
) -> GetNodeProviderRewardsCalculationResponse {
    let instruction_counter = telemetry::InstructionCounter::default();

    let res = NodeRewardsCanister::get_node_provider_rewards_calculation(&CANISTER, request);

    let encoded = Encode!(&res).unwrap();
    ic_cdk::println!(
        "get_node_provider_rewards_calculation response bytes: {:?}",
        encoded.len()
    );
    ic_cdk::println!("get_node_provider_rewards_calculation instructions: {:?} ", instruction_counter.sum());
    res
}
