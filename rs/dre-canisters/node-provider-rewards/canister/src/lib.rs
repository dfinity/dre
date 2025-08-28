use candid::{candid_method, encode_one, CandidType, Principal};
use chrono::Months;
use chrono::{DateTime, Days, Timelike, Utc};
use ic_cdk::spawn;
use ic_cdk_macros::*;
use ic_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_nervous_system_common::serve_metrics;
use ic_node_rewards_canister::canister::NodeRewardsCanister;
use ic_node_rewards_canister::registry_querier::RegistryQuerier;
use ic_node_rewards_canister::storage::{stable_btreemap_init, RegistryStoreStableMemoryBorrower, METRICS_MANAGER, VM};
use ic_node_rewards_canister::telemetry;
use ic_node_rewards_canister_api::provider_rewards_calculation::{GetNodeProviderRewardsCalculationRequest, GetNodeProviderRewardsCalculationResponse};
use ic_node_rewards_canister_api::providers_rewards::{GetNodeProvidersRewardsRequest, GetNodeProvidersRewardsResponse, NodeProvidersRewards};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_registry_canister_client::{get_decoded_value, CanisterRegistryClient, StableCanisterRegistryClient};
use ic_registry_keys::NODE_REWARDS_TABLE_KEY;
use node_provider_rewards_api::endpoints_deprecated::{NodeProviderRewardsCalculationArgs, RewardPeriodArgs, RewardsCalculatorResults};
use rewards_calculation::rewards_calculator::RewardsCalculatorInput;
use rewards_calculation::types::{DayUtc, RewardPeriod};
use rewards_calculation_deprecated::rewards_calculator::builder::RewardsCalculatorBuilder;
use rewards_calculation_deprecated::rewards_calculator::AlgoVersion;
use rewards_calculation_deprecated::types::{DayEnd, ProviderRewardableNodes};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use itertools::Itertools;
use rewards_calculation::rewards_calculator_results::NodeStatus;
use telemetry::QueryCallMeasurement;
use node_provider_rewards_api::endpoints::{GetNodesFRBySubnet, NodeDailyFR, SubnetNodesFR};

const HOUR_IN_SECONDS: u64 = 60 * 60;
const DAY_IN_SECONDS: u64 = HOUR_IN_SECONDS * 24;
const DFINITY_PROVIDER_ID: &str = "bvcsg-3od6r-jnydw-eysln-aql7w-td5zn-ay5m6-sibd2-jzojt-anwag-mqe"; // 42 nodes as of this checkin.
const ALLUSION_PROVIDER_ID: &str = "rbn2y-6vfsb-gv35j-4cyvy-pzbdu-e5aum-jzjg6-5b4n5-vuguf-ycubq-zae"; // 42 nodes as of this checkin.
const FRACTAL_LABS_PROVIDER_ID: &str = "wdjjk-blh44-lxm74-ojj43-rvgf4-j5rie-nm6xs-xvnuv-j3ptn-25t4v-6ae"; // 56 nodes as of this checkin.
const NODE_PROVIDERS_USED_DURING_CALCULATION_MEASUREMENT: [&str; 3] = [DFINITY_PROVIDER_ID, ALLUSION_PROVIDER_ID, FRACTAL_LABS_PROVIDER_ID];

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
const SYNC_INTERVAL_SECONDS: Duration = Duration::from_secs(60*60*2); // 4 hour
fn schedule_timers() {
    ic_cdk_timers::set_timer_interval(SYNC_INTERVAL_SECONDS, move || {
        spawn(async move {
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
                m.record_last_sync_instructions(instruction_counter.sum(), registry_sync_instructions, metrics_sync_instructions)
            });

            for np in NODE_PROVIDERS_USED_DURING_CALCULATION_MEASUREMENT {
                ic_cdk_timers::set_timer(SYNC_INTERVAL_SECONDS, || {
                    measure_get_node_provider_rewards_calculation_query(np)
                });
            }
        });
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
fn get_n_months_rewards_period(now: Option<DateTime<Utc>>, months: u32) -> (DayUtc, DayUtc) {
    let midnite = today_at_midnight(now).sub(Days::new(1));
    let ago = midnite.checked_sub_months(Months::new(months)).expect("UTC dates cannot have a nonexistent or unambiguous date after we subtract months, because UTC dates do not have daylight savings time, and there is no way this could be out of range.  See checked_sub_months() documentation.");

    let start_day = DayUtc::from(ago.timestamp_nanos_opt().unwrap() as u64);
    let end_day = DayUtc::from(midnite.timestamp_nanos_opt().unwrap() as u64);
    (start_day, end_day)
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

fn measure_get_node_provider_rewards_calculation_query(provider_id_s: &'static str) {
    // The argument is statically-lifetimed because all callers use static strings.
    let (from, to) = get_n_months_rewards_period(None, 1);
    let args = GetNodeProviderRewardsCalculationRequest {
        provider_id: ic_base_types::PrincipalId::from_str(provider_id_s).expect("The provider ID is a well-known ID.  This should never fail.").0,
        from_nanos: from.get(),
        to_nanos: to.get(),
        historical: false,
    };
    let measurement = measure_query_call(move || {
        let response = get_node_provider_rewards_calculation_v1(args);
        match response {
            Ok(rewards) => {Ok(rewards)}
            Err(error) => {
                ic_cdk::println!("error {}", error);
                Err("No rewards calculated".to_string())
            }
        }
    });
    telemetry::PROMETHEUS_METRICS.with_borrow_mut(|m| {
        m.record_node_provider_rewards_calculation_method(provider_id_s, measurement);
    });
}

#[query(hidden = true)]
fn http_request(request: HttpRequest) -> HttpResponse {
    match request.path() {
        "/metrics" => serve_metrics(|encoder| telemetry::PROMETHEUS_METRICS.with(|m| m.borrow().encode_metrics(encoder))),
        _ => HttpResponseBuilder::not_found().build(),
    }
}

#[query]
#[candid_method(query)]
fn get_subnets_list() -> Vec<Principal> {
    _get_subnets_list()
}
fn _get_subnets_list() -> Vec<Principal> {
    let metrics_manager = CANISTER.with(|canister| canister.borrow().get_metrics_manager());
    let metrics = metrics_manager.subnets_metrics.borrow();
    metrics.iter().map(|(k, _v)| k.subnet_id.unwrap().0).collect()
}

#[query]
#[candid_method(query)]
fn get_nodes_fr_by_subnet(request: RewardPeriodArgs) -> GetNodesFRBySubnet {
    _get_nodes_fr_by_subnet(request)
}

fn _get_nodes_fr_by_subnet(request: RewardPeriodArgs) -> Result<Vec<SubnetNodesFR>, String> {
    let rewards_calculation = CANISTER.with_borrow(|canister| {
        let request = GetNodeProvidersRewardsRequest {
            from_nanos: request.start_ts,
            to_nanos: request.end_ts,
        };
    canister.calculate_rewards::<RegistryStoreStableMemoryBorrower>(request, None)
    })?;

    let results = rewards_calculation
    .provider_results
        .into_values()
        .flat_map(|r| {
            r.nodes_results.into_iter().flat_map(|nr| {
                let node_id = nr.node_id.clone();
                nr.daily_results.into_iter()
                    .filter_map(move |dr| {
                        match dr.node_status {
                            NodeStatus::Assigned { node_metrics } => {
                                Some((node_metrics.subnet_assigned, node_metrics.subnet_assigned_fr, node_id, dr.day.into(), node_metrics.relative_fr))
                            }
                            _ => {
                                None
                            }
                        }
                    })
            })
        })
        .sorted()
        .chunk_by( |(subnet_id, subnet_fr, _, _, _)| (*subnet_id, *subnet_fr))
        .into_iter()
        .map(|((subnet_id, subnet_fr), group)| {
            let mut nodes_daily_fr = BTreeMap::new();
            for (_,_,node_id, day, relative_fr) in group {
                nodes_daily_fr.entry(node_id).or_insert_with(Vec::new).push((day, relative_fr.into()));
            }
            let nodes_daily_fr = nodes_daily_fr.into_iter().map(|(node_id, daily_relative_fr)| {
                NodeDailyFR {
                    node_id: node_id.get().0,
                    daily_relative_fr
                }
            }).collect();
            SubnetNodesFR {
                subnet_id: subnet_id.get().0,
                subnet_fr: subnet_fr.into(),
                nodes_daily_fr
            }
        }).collect();

    Ok(results)
}

#[query]
#[candid_method(query)]
fn get_node_provider_rewards_calculation_v1(request: GetNodeProviderRewardsCalculationRequest) -> GetNodeProviderRewardsCalculationResponse {
    NodeRewardsCanister::get_node_provider_rewards_calculation::<RegistryStoreStableMemoryBorrower>(
        &CANISTER, request,
    )
}

#[update]
#[candid_method(update)]
async fn get_node_providers_rewards(request: GetNodeProvidersRewardsRequest) -> GetNodeProvidersRewardsResponse {
    NodeRewardsCanister::get_node_providers_rewards::<RegistryStoreStableMemoryBorrower>(&CANISTER, request).await
}

// Deprecated method for backwards compatibility

fn rewards_calculator(reward_period: RewardPeriodArgs) -> Result<RewardsCalculatorInput, String> {
    let start_day = DayUtc::from(reward_period.start_ts);
    let end_day = DayUtc::from(reward_period.end_ts);
    let reward_period = RewardPeriod::new(start_day.into(), end_day.into()).map_err(|err| err.to_string())?;
    let metrics_manager = METRICS_MANAGER.with(|m| m.clone());
    let registry_store = REGISTRY_STORE.with(|m| m.clone());

    let rewards_table = get_decoded_value::<NodeRewardsTable>(&*registry_store, NODE_REWARDS_TABLE_KEY, registry_store.get_latest_version())
        .map_err(|err| format!("Failed to get rewards table from registry: {}", err))?
        .ok_or("Rewards table not found")?;
    let daily_metrics_by_subnet = metrics_manager
        .daily_metrics_by_subnet(reward_period.from, reward_period.to)
        .into_iter()
        .collect();
    let provider_rewardable_nodes =
        RegistryQuerier::get_rewardable_nodes_per_provider::<RegistryStoreStableMemoryBorrower>(&*registry_store, start_day.into(), end_day.into())
            .map_err(|err| format!("Failed to get rewardable nodes per provider: {}", err))?;

    Ok(RewardsCalculatorInput {
        reward_period,
        rewards_table,
        daily_metrics_by_subnet,
        provider_rewardable_nodes,
    })
}

#[query]
#[candid_method(query)]
fn get_node_provider_rewards_calculation(args: NodeProviderRewardsCalculationArgs) -> Result<RewardsCalculatorResults, String> {
    let RewardsCalculatorInput {
        reward_period,
        rewards_table,
        daily_metrics_by_subnet,
        provider_rewardable_nodes,
    } = rewards_calculator(args.reward_period)?;

    let reward_period = rewards_calculation_deprecated::rewards_calculator::RewardPeriod {
        from: rewards_calculation_deprecated::types::DayUTC::from(DayEnd::from(reward_period.from.get())),
        to: rewards_calculation_deprecated::types::DayUTC::from(DayEnd::from(reward_period.to.get())),
    };
    let daily_metrics_by_subnet = daily_metrics_by_subnet
        .into_iter()
        .map(|(key, metrics)| {
            (
                rewards_calculation_deprecated::types::SubnetMetricsDailyKey {
                    subnet_id: key.subnet_id,
                    day: rewards_calculation_deprecated::types::DayUTC::from(DayEnd::from(key.day.get())),
                },
                metrics
                    .into_iter()
                    .map(|m| rewards_calculation_deprecated::types::NodeMetricsDailyRaw {
                        node_id: m.node_id,
                        num_blocks_proposed: m.num_blocks_proposed,
                        num_blocks_failed: m.num_blocks_failed,
                    })
                    .collect(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let provider_rewardable_nodes = provider_rewardable_nodes
        .into_iter()
        .map(|(provider_id, nodes)| {
            let provider_nodes = ProviderRewardableNodes {
                provider_id,
                rewardable_nodes: nodes
                    .into_iter()
                    .map(|node| rewards_calculation_deprecated::types::RewardableNode {
                        node_id: node.node_id,
                        rewardable_days: node
                            .rewardable_days
                            .into_iter()
                            .map(|day| rewards_calculation_deprecated::types::DayUTC::from(DayEnd::from(day.get())))
                            .collect(),
                        region: rewards_calculation_deprecated::types::Region(node.region),
                        node_reward_type: node.node_reward_type,
                        dc_id: node.dc_id,
                    })
                    .collect(),
            };
            (provider_id, provider_nodes)
        })
        .collect::<BTreeMap<_, _>>();

    let provider_rewards_calculation = RewardsCalculatorBuilder {
        reward_period,
        rewards_table,
        daily_metrics_by_subnet,
        rewardable_nodes_per_provider: provider_rewardable_nodes,
    }
    .build()
    .map_err(|err| err.to_string())?
    .calculate_rewards_single_provider(args.provider_id, AlgoVersion::V0)
    .map_err(|err| err.to_string())?;

    provider_rewards_calculation.try_into()
}
