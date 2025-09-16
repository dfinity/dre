use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_node_rewards_canister::canister::NodeRewardsCanister;
use ic_node_rewards_canister::storage::{METRICS_MANAGER, RegistryStoreStableMemoryBorrower, REWARDABLE_NODES_CACHE};
use ic_node_rewards_canister::timer_tasks::DailySyncTask;
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
use std::cell::RefCell;
use std::sync::Arc;
use ic_nervous_system_timer_task::{RecurringAsyncTask, TimerTaskMetricsRegistry};
use ic_node_rewards_canister::pb::v1::RewardableNodesKey;
use ic_node_rewards_canister::{telemetry, MAX_PRINCIPAL_ID};
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
    DailySyncTask::new(&CANISTER, &METRICS_REGISTRY).schedule(&METRICS_REGISTRY);
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

#[query]
fn get_node_provider_rewards_calculation(
    request: GetNodeProviderRewardsCalculationRequest,
) -> GetNodeProviderRewardsCalculationResponse {
    let instruction_counter = telemetry::InstructionCounter::default();

    let res = NodeRewardsCanister::get_node_provider_rewards_calculation(&CANISTER, request);

    ic_cdk::println!("get_node_provider_rewards_calculation instructions: {:?}", instruction_counter.sum());
    res
}
