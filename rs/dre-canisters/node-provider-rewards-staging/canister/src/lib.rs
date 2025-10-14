use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_node_rewards_canister::canister::NodeRewardsCanister;
use ic_node_rewards_canister::storage::{METRICS_MANAGER, RegistryStoreStableMemoryBorrower};
use ic_node_rewards_canister::telemetry;
use ic_node_rewards_canister_api::monthly_rewards::{
    GetNodeProvidersMonthlyXdrRewardsRequest, GetNodeProvidersMonthlyXdrRewardsResponse,
};
use ic_node_rewards_canister_api::provider_rewards_calculation::{GetNodeProvidersRewardsCalculationRequest, GetNodeProvidersRewardsCalculationResponse};
use ic_node_rewards_canister_api::providers_rewards::{
    GetNodeProvidersRewardsRequest, GetNodeProvidersRewardsResponse,
};
use ic_registry_canister_client::StableCanisterRegistryClient;
use itertools::Itertools;
use std::cell::RefCell;
use std::sync::Arc;
use candid::Encode;
use ic_nervous_system_timer_task::{RecurringSyncTask, TimerTaskMetricsRegistry};
use ic_node_rewards_canister::timer_tasks::HourlySyncTask;

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
    static METRICS_REGISTRY: RefCell<TimerTaskMetricsRegistry> = RefCell::new(TimerTaskMetricsRegistry::default());

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


fn schedule_timers() {
    HourlySyncTask::new(&CANISTER).schedule(&METRICS_REGISTRY);

}
#[update]
async fn get_node_providers_monthly_xdr_rewards(
    request: GetNodeProvidersMonthlyXdrRewardsRequest,
) -> GetNodeProvidersMonthlyXdrRewardsResponse {
    NodeRewardsCanister::get_node_providers_monthly_xdr_rewards(&CANISTER, request).await
}

#[update]
fn get_node_providers_rewards(
    request: GetNodeProvidersRewardsRequest,
) -> GetNodeProvidersRewardsResponse {
    let instructions = telemetry::InstructionCounter::default();
    let res = NodeRewardsCanister::get_node_providers_rewards(&CANISTER, request);
    ic_cdk::println!("get_node_providers_rewards instructions: {:?}", instructions.sum());
    res
}

#[query]
fn get_node_providers_rewards_calculation(
    request: GetNodeProvidersRewardsCalculationRequest,
) -> GetNodeProvidersRewardsCalculationResponse {
    let instruction_counter = telemetry::InstructionCounter::default();

    let res = NodeRewardsCanister::get_node_providers_rewards_calculation(&CANISTER, request);

    let encoded = Encode!(&res).unwrap();
    ic_cdk::println!(
        "get_node_provider_rewards_calculation response bytes: {:?}",
        encoded.len()
    );
    ic_cdk::println!("get_node_provider_rewards_calculation instructions: {:?} ", instruction_counter.sum());
    res
}
