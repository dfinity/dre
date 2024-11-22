use crate::canister_data_provider::StableRegistryRecord;
use crate::types::{
    NodeMetricsStored, NodeMetricsStoredKey, NodeProviderRewards, NodeProviderRewardsKey, NodeRewardsMultiplierKey, RegistryKey,
    RewardsMultiplierStats, TimestampNanos,
};
use candid::Principal;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use itertools::Itertools;
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::collections::BTreeMap;

type Memory = VirtualMemory<DefaultMemoryImpl>;
pub type RegionNodeTypeCategory = (String, String);

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static NODES_METRICS: RefCell<StableBTreeMap<NodeMetricsStoredKey, NodeMetricsStored, Memory>> =
      RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
    ));

    static NODES_REWARDS_MULTIPLIER: RefCell<StableBTreeMap<NodeRewardsMultiplierKey, (u64, RewardsMultiplierStats), Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static NODE_PROVIDERS_REWARDS: RefCell<StableBTreeMap<NodeProviderRewardsKey, NodeProviderRewards, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static NODE_PROVIDERS_REWARDS_COMPUTATION_LOGS: RefCell<StableBTreeMap<NodeProviderRewardsKey, String, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));

    pub static REGISTRY_STORED: RefCell<StableBTreeMap<RegistryKey, Option<Vec<u8>>, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
    ));

    pub static REGISTRY: RefCell<StableVec<StableRegistryRecord, Memory>> =
        RefCell::new(StableVec::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
    ).unwrap());

    pub static TS_REGISTRY_VERSIONS: RefCell<StableBTreeMap<TimestampNanos, u64, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
    ));
}

lazy_static! {
    pub static ref MIN_STRING: String = String::from("");
    pub static ref MAX_STRING: String = String::from("\u{10FFFF}");
    static ref MIN_PRINCIPAL_ID: Principal = Principal::try_from(vec![]).expect("Unable to construct MIN_PRINCIPAL_ID.");
    static ref MAX_PRINCIPAL_ID: Principal =
        Principal::try_from(vec![0xFF_u8; Principal::MAX_LENGTH_IN_BYTES]).expect("Unable to construct MAX_PRINCIPAL_ID.");
}

pub fn insert_node_metrics(key: NodeMetricsStoredKey, value: NodeMetricsStored) {
    NODES_METRICS.with_borrow_mut(|nodes_metrics| nodes_metrics.insert(key, value));
}

pub fn latest_ts() -> Option<TimestampNanos> {
    NODES_METRICS
        .with_borrow(|nodes_metrics| nodes_metrics.last_key_value())
        .map(|((ts, _), _)| ts)
}

pub fn get_metrics_range(
    from_ts: TimestampNanos,
    to_ts: Option<TimestampNanos>,
    node_ids_filter: Option<&Vec<Principal>>,
) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    NODES_METRICS.with_borrow(|nodes_metrics| {
        let to_ts = to_ts.unwrap_or(u64::MAX);
        let node_in_range = nodes_metrics
            .range((from_ts, *MIN_PRINCIPAL_ID)..=(to_ts, *MAX_PRINCIPAL_ID))
            .collect_vec();

        if let Some(node_ids_filter) = node_ids_filter {
            node_in_range
                .into_iter()
                .filter(|((_, node_id), _)| node_ids_filter.contains(node_id))
                .collect_vec()
        } else {
            node_in_range
        }
    })
}

pub fn latest_metrics(nodes_principal: &[Principal]) -> BTreeMap<Principal, NodeMetricsStored> {
    let mut latest_metrics = BTreeMap::new();
    NODES_METRICS.with_borrow(|nodes_metrics| {
        for ((_, principal), value) in nodes_metrics.iter() {
            if nodes_principal.contains(&principal) {
                latest_metrics.insert(principal, value);
            }
        }
    });

    latest_metrics
}

pub fn store_node_rewards_multiplier(
    rewards_ts: u64,
    node_provider: Principal,
    node_id: Principal,
    multiplier: u64,
    multiplier_stats: crate::types::RewardsMultiplierStats,
) {
    NODES_REWARDS_MULTIPLIER.with_borrow_mut(|node_rewards_multiplier| {
        node_rewards_multiplier.insert((rewards_ts, node_provider, node_id), (multiplier, multiplier_stats))
    });
}

pub fn store_node_provider_rewards(rewards_ts: u64, node_provider: Principal, node_provider_rewards: NodeProviderRewards) {
    NODE_PROVIDERS_REWARDS
        .with_borrow_mut(|node_providers_rewards| node_providers_rewards.insert((rewards_ts, node_provider), node_provider_rewards));
}

pub fn store_node_provider_logs(rewards_ts: u64, node_provider: Principal, log: String) {
    NODE_PROVIDERS_REWARDS_COMPUTATION_LOGS
        .with_borrow_mut(|node_providers_rewards_computation_logs| node_providers_rewards_computation_logs.insert((rewards_ts, node_provider), log));
}

pub(crate) fn wipe() {
    REGISTRY_STORED.with_borrow_mut(|mem| mem.clear_new());
    TS_REGISTRY_VERSIONS.with_borrow_mut(|mem| mem.clear_new());
}
