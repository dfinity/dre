use candid::Encode;
use ic_cdk::{init, post_upgrade, pre_upgrade, query, update};
use ic_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_nervous_system_timer_task::{RecurringSyncTask, TimerTaskMetricsRegistry};
use ic_node_rewards_canister::canister::NodeRewardsCanister;
use ic_node_rewards_canister::storage::{RegistryStoreStableMemoryBorrower, LAST_DAY_SYNCED, METRICS_MANAGER};
use ic_node_rewards_canister::telemetry;
use ic_node_rewards_canister::telemetry::PROMETHEUS_METRICS;
use ic_node_rewards_canister::timer_tasks::{GetNodeProvidersRewardsInstructionsExporter, HourlySyncTask};
use ic_node_rewards_canister_api::monthly_rewards::{
    GetNodeProvidersMonthlyXdrRewardsRequest, GetNodeProvidersMonthlyXdrRewardsResponse,
};
use ic_node_rewards_canister_api::provider_rewards_calculation::{GetNodeProvidersRewardsCalculationRequest, GetNodeProvidersRewardsCalculationResponse};
use ic_node_rewards_canister_api::providers_rewards::{
    GetNodeProvidersRewardsRequest, GetNodeProvidersRewardsResponse,
};
use ic_registry_canister_client::StableCanisterRegistryClient;
use std::cell::RefCell;
use std::sync::Arc;

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

        RefCell::new(NodeRewardsCanister::new(registry_store, metrics_manager, &LAST_DAY_SYNCED))
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


pub fn schedule_timers() {
    HourlySyncTask::new(&CANISTER).schedule(&METRICS_REGISTRY);
    GetNodeProvidersRewardsInstructionsExporter::new(&CANISTER).schedule(&METRICS_REGISTRY);
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

fn encode_metrics(w: &mut ic_metrics_encoder::MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    METRICS_REGISTRY.with_borrow(|registry| registry.encode("node_rewards", w))?;
    PROMETHEUS_METRICS.with_borrow(|p| p.encode_metrics(w))
}

#[query(
    hidden = true,
    decode_with = "candid::decode_one_with_decoding_quota::<1000000,_>"
)]
fn http_request(request: HttpRequest) -> HttpResponse {
    match request.path() {
        "/metrics" => {
            let mut w = ic_metrics_encoder::MetricsEncoder::new(
                vec![],
                ic_cdk::api::time() as i64 / 1_000_000,
            );

            match encode_metrics(&mut w) {
                Ok(_) => HttpResponseBuilder::ok()
                    .header("Content-Type", "text/plain; version=0.0.4")
                    .header("Cache-Control", "no-store")
                    .with_body_and_content_length(w.into_inner())
                    .build(),
                Err(err) => {
                    HttpResponseBuilder::server_error(format!("Failed to encode metrics: {err}"))
                        .build()
                }
            }
        }
        _ => HttpResponseBuilder::not_found().build(),
    }
}
