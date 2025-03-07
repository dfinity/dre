use crate::metrics::{MetricsManager, MetricsManagerData, StableLastTimestampPerSubnet, StableSubnetsMetrics};
use crate::metrics_types::{SubnetIdStored, SubnetMetricsStored, SubnetMetricsStoredKey, TimestampNanos};
use crate::registry_store::{CanisterRegistryStore, RegistryData, StableLocalRegistry};
use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use ic_base_types::SubnetId;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use std::cell::RefCell;
use std::collections::HashSet;

pub type RegistryStoreInstance = CanisterRegistryStore<State>;
pub type MetricsManagerInstance = MetricsManager<State>;
pub type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static STATE: RefCell<State> = RefCell::new(State::init());
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}

const REGISTRY_STORE_LOCAL_REGISTRY_MEMORY_ID: MemoryId = MemoryId::new(0);

const METRICS_MANAGER_SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const METRICS_MANAGER_LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

pub struct State {
    subnets_to_retry: HashSet<SubnetId>,
    local_registry: StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>,
    subnets_metrics: StableBTreeMap<SubnetMetricsStoredKey, SubnetMetricsStored, VM>,
    last_timestamp_per_subnet: StableBTreeMap<SubnetIdStored, TimestampNanos, VM>,
}

impl State {
    fn init() -> Self {
        let local_registry = with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(REGISTRY_STORE_LOCAL_REGISTRY_MEMORY_ID)));

        let subnets_metrics = with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(METRICS_MANAGER_SUBNETS_METRICS_MEMORY_ID)));

        let last_timestamp_per_subnet = with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(METRICS_MANAGER_LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID)));

        let subnets_to_retry_stored: StableVec<SubnetIdStored, VM> =
            with_memory_manager(|mgr| StableVec::init(mgr.get(METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID)))
                .expect("Failed to initialize subnets_to_retry_stored");
        let subnets_to_retry: HashSet<SubnetId> = subnets_to_retry_stored.iter().map(|subnet_id_stored| subnet_id_stored.0).collect();

        Self {
            subnets_to_retry,
            local_registry,
            subnets_metrics,
            last_timestamp_per_subnet,
        }
    }
}

impl RegistryData for State {
    fn with_local_registry<R>(f: impl FnOnce(&StableLocalRegistry) -> R) -> R {
        STATE.with_borrow(|state| f(&state.local_registry))
    }

    fn with_local_registry_mut<R>(f: impl FnOnce(&mut StableLocalRegistry) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.local_registry))
    }
}

impl MetricsManagerData for State {
    fn with_subnets_to_retry<R>(f: impl FnOnce(&HashSet<SubnetId>) -> R) -> R {
        STATE.with_borrow(|state| f(&state.subnets_to_retry))
    }

    fn with_subnets_to_retry_mut<R>(f: impl FnOnce(&mut HashSet<SubnetId>) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.subnets_to_retry))
    }

    fn with_subnets_metrics<R>(f: impl FnOnce(&StableSubnetsMetrics) -> R) -> R {
        STATE.with_borrow(|state| f(&state.subnets_metrics))
    }

    fn with_subnets_metrics_mut<R>(f: impl FnOnce(&mut StableSubnetsMetrics) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.subnets_metrics))
    }

    fn with_last_timestamp_per_subnet<R>(f: impl FnOnce(&StableLastTimestampPerSubnet) -> R) -> R {
        STATE.with_borrow(|state| f(&state.last_timestamp_per_subnet))
    }

    fn with_last_timestamp_per_subnet_mut<R>(f: impl FnOnce(&mut StableLastTimestampPerSubnet) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.last_timestamp_per_subnet))
    }
}

pub fn pre_upgrade() {
    let subnets_to_retry = STATE.with_borrow(|state| state.subnets_to_retry.clone());
    let subnets_to_retry_stored: StableVec<SubnetIdStored, VM> =
        with_memory_manager(|mgr| StableVec::new(mgr.get(METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID)))
            .expect("Failed to initialize subnets_to_retry_stored");

    for subnet in subnets_to_retry {
        subnets_to_retry_stored
            .push(&SubnetIdStored(subnet))
            .expect("Failed to push subnet to subnets_to_retry_stored");
    }
}
