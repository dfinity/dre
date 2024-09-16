use candid::Principal;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::BTreeMap;

use trustworthy_node_metrics_types::types::{NodeMetadata, NodeMetadataStored, NodeMetricsStored, NodeMetricsStoredKey, NodeProviderMapping, TimestampNanos};

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

pub fn insert_node_provider(key: Principal, value: Principal) {
    NODE_PROVIDER_MAP.with(|p| p.borrow_mut().insert(key, value));
}

pub fn get_node_provider(node_principal: &Principal) -> Option<Principal> {
    NODE_PROVIDER_MAP.with_borrow(|np_map| np_map.get(node_principal))
}

pub fn get_node_principals(node_provider: &Principal) -> Vec<Principal> {
    NODE_PROVIDER_MAP.with_borrow(|np_map| {
        np_map
            .iter()
            .filter_map(|(node_id, node_p)| if &node_p == node_provider { Some(node_id) } else { None })
            .collect_vec()
    })
}

pub fn get_node_provider_mapping() -> Vec<NodeProviderMapping> {
    NODE_PROVIDER_MAP.with_borrow(|np_map| {
        np_map
            .iter()
            .map(|(node_id, node_provider_id)| NodeProviderMapping { node_id, node_provider_id })
            .collect_vec()
    })
}


pub fn nodes_metadata() -> Vec<NodeMetadata> {
    NODE_METADATA.with_borrow(|node_metadata| {
        node_metadata.iter().map(|(node_id, node_metadata_stored)| {
            NodeMetadata {
                node_id,
                node_provider_id: node_metadata_stored.node_provider_id,
                node_provider_name: node_metadata_stored.node_provider_name
            }
        })
        .collect_vec()
    })
}
