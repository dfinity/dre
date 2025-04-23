use crate::storage::{METRICS_MANAGER, REGISTRY_STORE};
use candid::candid_method;
use ic_canisters_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_cdk_macros::*;
use ic_nervous_system_common::serve_metrics;
use node_provider_rewards_api::endpoints::{NodeProviderRewardsCalculationArgs, NodeProvidersRewards, RewardPeriodArgs, RewardsCalculatorResults};
use rewards_calculation::rewards_calculator::RewardsCalculator;
use rewards_calculation::types::RewardPeriod;
use std::collections::BTreeMap;
use rewards_calculation::rewards_calculator;
use rewards_calculation::rewards_calculator::builder::RewardsCalculatorBuilder;

mod metrics;
mod metrics_types;
mod registry;
mod storage;
mod telemetry;

const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;

/// Sync the local registry and subnets metrics with remote
///
/// - Sync local registry stored from the remote registry canister
/// - Sync subnets metrics from the management canister of the different subnets
async fn sync_all() {
    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_start());
    let registry_store = REGISTRY_STORE.with(|m| m.clone());

    match registry_store.schedule_registry_sync().await {
        Ok(_) => {
            let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
            let subnets_list = registry_store.subnets_list();

            metrics_manager.update_subnets_metrics(subnets_list).await;
            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_success());
            ic_cdk::println!("Successfully synced subnets metrics and local registry");
        }
        Err(e) => {
            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_end());
            ic_cdk::println!("Failed to sync local registry: {:?}", e)
        }
    }
}


fn sum(x: i32, y: i32) -> i32 {
    x + y
}
// Explicit coercion to `fn` type is required...

fn setup_timers() {
    let op: fn(i32, i32) -> i32 = sum;

    // Next 1 AM UTC timestamp
    let next_utc_1am_sec = DAY_IN_SECONDS + HOUR_IN_SECONDS - (ic_cdk::api::time() / 1_000_000_000) % DAY_IN_SECONDS;
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(next_utc_1am_sec), || {
        ic_cdk::spawn(sync_all());

        // Reschedule for the next day
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(DAY_IN_SECONDS), || ic_cdk::spawn(sync_all()));
    });

    // Retry subnets fetching every hour
    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HOUR_IN_SECONDS), || {
        ic_cdk::spawn(async {
            let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
            metrics_manager.retry_failed_subnets().await;
        });
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

#[query(hidden = true, decoding_quota = 10000)]
fn http_request(request: HttpRequest) -> HttpResponse {
    match request.path() {
        "/metrics" => serve_metrics(|encoder| telemetry::PROMETHEUS_METRICS.with(|m| telemetry::encode_metrics(&m.borrow(), encoder))),
        _ => HttpResponseBuilder::not_found().build(),
    }
}

fn rewards_calculator(reward_period: RewardPeriodArgs) -> Result<RewardsCalculator, String> {
    let reward_period = RewardPeriod::new(reward_period.start_ts, reward_period.end_ts).map_err(|err| err.to_string())?;
    let start_ts = reward_period.from.get();
    let end_ts = reward_period.to.get();

    let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
    let registry_store = REGISTRY_STORE.with(|m| m.clone());

    let rewards_table = registry_store.get_rewards_table();
    let daily_metrics_by_subnet = metrics_manager.daily_metrics_by_subnet(start_ts, end_ts);
    let rewardable_nodes_per_provider = registry_store.get_rewardable_nodes_per_provider(start_ts, end_ts).map_err(|err| err.to_string())?;

    let rewards_calculator = RewardsCalculatorBuilder {
        reward_period,
        rewards_table,
        daily_metrics_by_subnet,
        rewardable_nodes_per_provider,
    }.build().map_err(|err| err.to_string())?;

    Ok(rewards_calculator)
}

#[query]
#[candid_method(query)]
fn get_node_providers_rewards(args: RewardPeriodArgs) -> Result<NodeProvidersRewards, String> {
    let calculator= rewards_calculator(args)?;
    let rewards_per_provider = calculator.calculate_rewards_per_provider().into_iter()
        .map(|(provider_id, rewards_calculation)| (provider_id, rewards_calculation.rewards_total))
        .collect::<BTreeMap<_, _>>();

    rewards_per_provider.try_into()
}

#[query]
#[candid_method(query)]
fn get_node_provider_rewards_calculation(args: NodeProviderRewardsCalculationArgs) -> Result<RewardsCalculatorResults, String> {
    let calculator= rewards_calculator(args.reward_period)?;
    let provider_rewards_calculation = calculator.calculate_rewards_single_provider(args.provider_id).map_err(|err| err.to_string())?;

    provider_rewards_calculation.try_into()
}
