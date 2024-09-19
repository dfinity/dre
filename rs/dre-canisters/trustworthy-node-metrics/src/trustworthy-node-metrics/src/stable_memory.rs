use candid::Principal;
use ic_protobuf::registry::node_rewards::v2::NodeRewardRates;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::BTreeMap;

use trustworthy_node_metrics_types::types::{
    NodeMetadata, NodeMetadataStored, NodeMetadataStoredV2, NodeMetricsStored, NodeMetricsStoredKey, NodeRewardRatesStored, TimestampNanos
};

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static NODE_METRICS_MAP: RefCell<StableBTreeMap<NodeMetricsStoredKey, NodeMetricsStored, Memory>> =
      RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
    ));

    static NODE_PROVIDER_MAP: RefCell<StableBTreeMap<Principal, Principal, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
    static NODE_PROVIDER_MAP_V1: RefCell<StableBTreeMap<Principal, Principal, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));
    static NODE_METADATA: RefCell<StableBTreeMap<Principal, NodeMetadataStored, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
    static NODE_METADATA_V2: RefCell<StableBTreeMap<Principal, NodeMetadataStoredV2, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
    ));

    static REWARDS_TABLE: RefCell<StableBTreeMap<String, NodeRewardRatesStored, Memory>> =
        RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
    ));
}

pub fn insert_node_metrics(key: NodeMetricsStoredKey, value: NodeMetricsStored) {
    NODE_METRICS_MAP.with(|p| p.borrow_mut().insert(key, value));
}

pub fn latest_ts() -> Option<TimestampNanos> {
    NODE_METRICS_MAP.with(|p| p.borrow().last_key_value()).map(|((ts, _), _)| ts)
}

pub fn get_metrics_range(
    from_ts: TimestampNanos,
    to_ts: Option<TimestampNanos>,
    node_ids_filter: Option<Vec<Principal>>,
) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    NODE_METRICS_MAP.with(|p| {
        let to_ts = to_ts.unwrap_or(u64::MAX);
        let node_in_range = p
            .borrow()
            .range((from_ts, Principal::anonymous())..=(to_ts, Principal::anonymous()))
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
    NODE_METRICS_MAP.with(|p| {
        for ((_, principal), value) in p.borrow().iter() {
            if nodes_principal.contains(&principal) {
                latest_metrics.insert(principal, value);
            }
        }
    });

    latest_metrics
}

pub fn get_node_provider(node_principal: &Principal) -> Option<Principal> {
    NODE_METADATA.with_borrow(|node_metadata| node_metadata.get(node_principal).map(|metadata| metadata.node_provider_id))
}

pub fn get_node_principals(node_provider: &Principal) -> Vec<Principal> {
    NODE_METADATA.with_borrow(|node_metadata| {
        node_metadata
            .iter()
            .filter_map(|(node_id, node_metadata)| {
                if &node_metadata.node_provider_id == node_provider {
                    Some(node_id)
                } else {
                    None
                }
            })
            .collect_vec()
    })
}

pub fn nodes_metadata() -> Vec<NodeMetadata> {
    NODE_METADATA_V2.with_borrow(|node_metadata| {
        node_metadata
            .iter()
            .map(|(node_id, node_metadata_stored)| NodeMetadata {
                node_id,
                node_metadata_stored
            })
            .collect_vec()
    })
}

pub fn insert_rewards_rates(area: String, rewards_rates: NodeRewardRates) {
    REWARDS_TABLE.with_borrow_mut(|rewards_table| rewards_table.insert(area, NodeRewardRatesStored { rewards_rates }));
}

pub fn insert_metadata_v2(node_id: Principal, node_operator_id: Principal, node_provider_id: Principal, dc_id: String, region: String, node_type: String) {
    NODE_METADATA_V2.with_borrow_mut(|node_metadata| {
        node_metadata.insert(
            node_id,
            NodeMetadataStoredV2 {
                region: region.to_string(),
                node_type: node_type.to_string(),
                dc_id: dc_id.to_string(),
                node_operator_id,
                node_provider_id,
                node_provider_name: None
            },
        )
    });
}

pub fn node_types_count(node_operator_id: Principal) -> Option<BTreeMap<String, i32>> {
    let mut node_types_count = BTreeMap::new();

    NODE_METADATA_V2.with_borrow(|node_metadata| {
        let operator_metadata = node_metadata.iter().filter(|(_, metadata)| metadata.node_operator_id == node_operator_id).collect_vec();

        for (_, metadata) in operator_metadata {
            let counter = node_types_count.entry(metadata.node_type).or_insert(0);
            *counter += 1;
        }
    });

    if node_types_count.is_empty() {
        None
    } else {
        Some(node_types_count)
    }
}
