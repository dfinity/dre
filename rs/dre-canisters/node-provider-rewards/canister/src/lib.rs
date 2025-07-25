use crate::storage::{METRICS_MANAGER, REGISTRY_STORE};
use candid::{candid_method, encode_one, CandidType};
use chrono::Months;
use chrono::{DateTime, Days, Duration, Timelike, Utc};
use ic_cdk_macros::*;
use ic_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_nervous_system_common::serve_metrics;
use ic_types::PrincipalId;
use node_provider_rewards_api::endpoints::{
    NodeProviderRewardsCalculationArgs, NodeProvidersRewards, RewardPeriodArgs, RewardsCalculatorResults, RewardsCalculatorResultsV1,
};
use rewards_calculation::rewards_calculator::builder::RewardsCalculatorBuilder;
use rewards_calculation::rewards_calculator::{AlgoVersion, RewardsCalculator};
use rewards_calculation::types::RewardPeriod;
use std::collections::BTreeMap;
use std::ops::{Add, Sub};
use std::str::FromStr;
use telemetry::QueryCallMeasurement;

mod metrics;
mod metrics_types;
mod registry;
mod storage;
mod telemetry;

const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;
const DFINITY_PROVIDER_ID: &str = "bvcsg-3od6r-jnydw-eysln-aql7w-td5zn-ay5m6-sibd2-jzojt-anwag-mqe"; // 42 nodes as of this checkin.
const ALLUSION_PROVIDER_ID: &str = "rbn2y-6vfsb-gv35j-4cyvy-pzbdu-e5aum-jzjg6-5b4n5-vuguf-ycubq-zae"; // 42 nodes as of this checkin.
const FRACTAL_LABS_PROVIDER_ID: &str = "wdjjk-blh44-lxm74-ojj43-rvgf4-j5rie-nm6xs-xvnuv-j3ptn-25t4v-6ae"; // 56 nodes as of this checkin.
const NODE_PROVIDERS_USED_DURING_CALCULATION_MEASUREMENT: [&str; 3] = [DFINITY_PROVIDER_ID, ALLUSION_PROVIDER_ID, FRACTAL_LABS_PROVIDER_ID];

/// Sync the local registry and subnets metrics with remote
///
/// - Sync local registry stored from the remote registry canister
/// - Sync subnets metrics from the management canister of the different subnets
async fn sync_all() {
    let mut instruction_counter = telemetry::InstructionCounter::default();
    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_start());
    let registry_store = REGISTRY_STORE.with(|m| m.clone());

    instruction_counter.lap();
    let result = registry_store.schedule_registry_sync().await;
    let registry_sync_instructions = instruction_counter.lap();

    let mut subnet_list_instructions: u64 = 0;
    let mut update_subnet_metrics_instructions: u64 = 0;

    match result {
        Ok(_) => {
            let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
            instruction_counter.lap(); // Reset the lap time.
            let subnets_list = registry_store.subnets_list();
            subnet_list_instructions = instruction_counter.lap();
            metrics_manager.update_subnets_metrics(subnets_list).await;
            update_subnet_metrics_instructions = instruction_counter.lap();

            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_success());
            ic_cdk::println!("Successfully synced subnets metrics and local registry");
        }
        Err(e) => {
            telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| m.mark_last_sync_failure());
            ic_cdk::println!("Failed to sync local registry: {:?}", e)
        }
    }

    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| {
        m.record_last_sync_instructions(
            instruction_counter.sum(),
            registry_sync_instructions,
            subnet_list_instructions,
            update_subnet_metrics_instructions,
        )
    });
}

/// Get the beginning of this hour (if now is None) or the beginning of the hour
/// for the passed date/time.
fn start_of_this_hour(now: Option<DateTime<Utc>>) -> DateTime<Utc> {
    now.unwrap_or(DateTime::from_timestamp_nanos(ic_cdk::api::time().try_into().unwrap()))
    .with_nanosecond(0).expect("Zeroing out nanoseconds should never fail.")
    .with_second(0)
    .expect("An i64 with nanosecond precision can span a range of ~584 years. Because all values can be represented as a DateTime this method never fails.")
    .with_minute(0)
    .expect("UTC datetimes with minute and second zero always exist.  Hence the unwrap.")
}

/// Get midnight of today (if now is None) or the midnight of the date/time passed.
fn today_at_midnight(now: Option<DateTime<Utc>>) -> DateTime<Utc> {
    start_of_this_hour(now).with_hour(0).expect("Midnight always exists in UTC time.")
}

/// Get an interval that ends at the beginning of the day and starts N months before that.
/// The first value in the return tuple is the start of the interval.  The second is the end.
/// The supplied value is used the reference date/time (if None, uses current date/time).
///
/// If the supplied or current date/time falls in an end of the month day, and the target
/// month (N months before) has fewer days than the supplied date/time, this code does the
/// right thing and computes the end of that month.  The wrong thing would be to blindly
/// subtract 62 days or something equally arbitrary.  Example of the right thing:
///
/// * supplied date/time: 2025-04-30T03:01:00
/// * returned interval: (2025-02-28T00:00:00 -- 2025-04-30T00:00:00)
fn get_n_months_rewards_period(now: Option<DateTime<Utc>>, months: u32) -> RewardPeriodArgs {
    let midnite = today_at_midnight(now).sub(Days::new(1));
    let ago = midnite.checked_sub_months(Months::new(months)).expect("UTC dates cannot have a nonexistent or unambiguous date after we subtract months, because UTC dates do not have daylight savings time, and there is no way this could be out of range.  See checked_sub_months() documentation.");
    RewardPeriodArgs {
        start_ts: ago.timestamp_nanos_opt().unwrap() as u64,
        end_ts: midnite.timestamp_nanos_opt().unwrap() as u64,
    }
}

/// Compute the duration left until either today or tomorrow at 1AM (whichever is earliest).
fn time_left_for_next_1am(now: Option<DateTime<Utc>>) -> std::time::Duration {
    let really_now = now.unwrap_or(DateTime::from_timestamp_nanos(ic_cdk::api::time().try_into().unwrap()));
    let today_midnight_utc = today_at_midnight(Some(really_now));
    let tomorrow_1am = today_midnight_utc
        .checked_add_days(Days::new(1))
        .expect("Tomorrow in UTC always exists.")
        .add(Duration::hours(1));
    (tomorrow_1am - really_now)
        .to_std()
        .expect("Tomorrow 1AM minus right now should never be out of range.")
}

fn measure_query_call<Q, O, E>(f: Q) -> QueryCallMeasurement
where
    Q: FnOnce() -> Result<O, E>,
    O: CandidType,
    E: CandidType,
{
    let instruction_counter = telemetry::InstructionCounter::default();
    let response = f();
    let success = response.is_ok();
    let response_size_bytes: usize = encode_one(response).expect("Failed to encode").len();
    let instructions = instruction_counter.sum();
    // REVIEWER: here we count the instructions of the encoding too, reasoning that the VM
    // will also count the instructions it takes to encode the response towards the budget
    // of instructions that the canister gets to respond to the query call.
    (success, instructions, response_size_bytes)
}

fn measure_get_node_providers_rewards_query() {
    let reward_period = get_n_months_rewards_period(None, 2);
    let measurement = measure_query_call(move || get_node_providers_rewards(reward_period));
    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| {
        m.record_node_provider_rewards_method(measurement);
    });
}

fn measure_get_node_provider_rewards_calculation_query(provider_id_s: &'static str) {
    // The argument is statically-lifetimed because all callers use static strings.
    let args = NodeProviderRewardsCalculationArgs {
        provider_id: PrincipalId::from_str(provider_id_s).expect("The provider ID is a well-known ID.  This should never fail."),
        reward_period: get_n_months_rewards_period(None, 1),
    };
    let measurement = measure_query_call(move || get_node_provider_rewards_calculation(args));
    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| {
        m.record_node_provider_rewards_calculation_method(provider_id_s, measurement);
    });
}

fn setup_timers() {
    // I had to rewrite this to compute the correct remaining time until next 1AM.
    // It is simply not true that one can get a midnight from the modulo of seconds since
    // the UNIX epoch (as it was being done before).  Leap seconds are a thing.
    ic_cdk_timers::set_timer(time_left_for_next_1am(None), || {
        // It's 1AM since the canister was installed or upgraded.
        // Schedule a repeat timer to run sync_all() every 24 hours.
        // Sadly we ignore leap seconds here.
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(DAY_IN_SECONDS), || ic_cdk::spawn(sync_all()));

        // Spawn a sync_all() right now.
        ic_cdk::spawn(sync_all());

        // Hourly timers after first sync.  One for rewards query, and N for rewards calculation query.
        ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HOUR_IN_SECONDS), measure_get_node_providers_rewards_query);
        for np in NODE_PROVIDERS_USED_DURING_CALCULATION_MEASUREMENT {
            ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HOUR_IN_SECONDS), || {
                measure_get_node_provider_rewards_calculation_query(np)
            });
        }
    });

    // Hourly timers.
    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(HOUR_IN_SECONDS), || {
        // Retry subnets fetching every hour.
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

#[query(hidden = true)]
fn http_request(request: HttpRequest) -> HttpResponse {
    match request.path() {
        "/metrics" => serve_metrics(|encoder| telemetry::PROMETHEUS_METRICS.with(|m| m.borrow().encode_metrics(encoder))),
        _ => HttpResponseBuilder::not_found().build(),
    }
}

fn rewards_calculator(reward_period: RewardPeriodArgs) -> Result<RewardsCalculator, String> {
    let reward_period = RewardPeriod::new(reward_period.start_ts, reward_period.end_ts).map_err(|err| err.to_string())?;
    let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
    let registry_store = REGISTRY_STORE.with(|m| m.clone());

    let rewards_table = registry_store.get_rewards_table();
    let daily_metrics_by_subnet = metrics_manager.daily_metrics_by_subnet(reward_period.from, reward_period.to);

    let rewardable_nodes_per_provider = registry_store
        .get_rewardable_nodes_per_provider(reward_period.from, reward_period.to)
        .map_err(|err| err.to_string())?;

    let rewards_calculator = RewardsCalculatorBuilder {
        reward_period,
        rewards_table,
        daily_metrics_by_subnet,
        rewardable_nodes_per_provider,
    }
    .build()
    .map_err(|err| err.to_string())?;

    Ok(rewards_calculator)
}

#[query]
#[candid_method(query)]
fn get_node_providers_rewards(args: RewardPeriodArgs) -> Result<NodeProvidersRewards, String> {
    let calculator = rewards_calculator(args)?;
    let rewards_per_provider = calculator
        .calculate_rewards_per_provider()
        .into_iter()
        .map(|(provider_id, rewards_calculation)| (provider_id, rewards_calculation.rewards_total))
        .collect::<BTreeMap<_, _>>();

    rewards_per_provider.try_into()
}

#[query]
#[candid_method(query)]
fn get_node_provider_rewards_calculation(args: NodeProviderRewardsCalculationArgs) -> Result<RewardsCalculatorResults, String> {
    let calculator = rewards_calculator(args.reward_period)?;
    let provider_rewards_calculation = calculator
        .calculate_rewards_single_provider(args.provider_id, AlgoVersion::V0)
        .map_err(|err| err.to_string())?;

    provider_rewards_calculation.try_into()
}

#[query]
#[candid_method(query)]
fn get_node_provider_rewards_calculation_v1(args: NodeProviderRewardsCalculationArgs) -> Result<RewardsCalculatorResultsV1, String> {
    let calculator = rewards_calculator(args.reward_period)?;
    let provider_rewards_calculation = calculator
        .calculate_rewards_single_provider(args.provider_id, AlgoVersion::V1)
        .map_err(|err| err.to_string())?;

    provider_rewards_calculation.try_into()
}
