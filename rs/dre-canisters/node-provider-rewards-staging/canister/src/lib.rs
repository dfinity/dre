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
    REWARDABLE_NODES_CACHE.with(|cache| {
        let first_reg = cache.borrow().first_key_value().unwrap().0.registry_version;
        let first_key_remove = RewardableNodesKey {
            registry_version: first_reg,
            provider_id: None
        };
        let last_key_remove = RewardableNodesKey {
            registry_version: first_reg,
            provider_id: Some(MAX_PRINCIPAL_ID)
        };
        let keys = cache.borrow().range(first_key_remove..=last_key_remove)
            .map(|x| x.0).collect_vec();

        for key_remove in keys {
            cache.borrow_mut().remove(&key_remove);
        }
    });
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

#[update]
fn get_node_provider_rewards_calculation(
    request: GetNodeProviderRewardsCalculationRequest,
) -> GetNodeProviderRewardsCalculationResponse {
    let instruction_counter = telemetry::InstructionCounter::default();

    let res = NodeRewardsCanister::get_node_provider_rewards_calculation(&CANISTER, request);

    ic_cdk::println!("get_node_provider_rewards_calculation instructions: {:?}", instruction_counter.sum());
    res
}
