use crate::metrics::{MetricsManagerData, RetryCount, StableLastTimestampPerSubnet, StableSubnetsMetrics, StableSubnetsToRetry, TimestampNanos};
use crate::metrics_types::{StorableSubnetMetrics, StorableSubnetMetricsKey, SubnetIdStored};
use crate::registry_store::{RegistryStoreData, StableLocalRegistry};
use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use std::cell::RefCell;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static STATE: RefCell<State> = RefCell::new(State::init());
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}
fn stable_btreemap_init<K: Storable + Clone + Ord, V: Storable>(memory_id: MemoryId) -> StableBTreeMap<K, V, VM> {
    with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(memory_id)))
}

const LOCAL_REGISTRY_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

pub struct State {
    subnets_to_retry: StableBTreeMap<SubnetIdStored, RetryCount, VM>,
    local_registry: StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>,
    subnets_metrics: StableBTreeMap<StorableSubnetMetricsKey, StorableSubnetMetrics, VM>,
    last_timestamp_per_subnet: StableBTreeMap<SubnetIdStored, TimestampNanos, VM>,
}

impl State {
    fn init() -> Self {
        State {
            subnets_to_retry: stable_btreemap_init(SUBNETS_TO_RETRY_MEMORY_ID),
            local_registry: stable_btreemap_init(LOCAL_REGISTRY_MEMORY_ID),
            subnets_metrics: stable_btreemap_init(SUBNETS_METRICS_MEMORY_ID),
            last_timestamp_per_subnet: stable_btreemap_init(LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID),
        }
    }
}

impl RegistryStoreData<VM> for State {
    fn with_local_registry<R>(f: impl FnOnce(&StableLocalRegistry<VM>) -> R) -> R {
        STATE.with_borrow(|state| f(&state.local_registry))
    }

    fn with_local_registry_mut<R>(f: impl FnOnce(&mut StableLocalRegistry<VM>) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.local_registry))
    }
}

impl MetricsManagerData<VM> for State {
    fn with_subnets_to_retry<R>(f: impl FnOnce(&StableSubnetsToRetry<VM>) -> R) -> R {
        STATE.with_borrow(|state| f(&state.subnets_to_retry))
    }

    fn with_subnets_to_retry_mut<R>(f: impl FnOnce(&mut StableSubnetsToRetry<VM>) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.subnets_to_retry))
    }

    fn with_subnets_metrics<R>(f: impl FnOnce(&StableSubnetsMetrics<VM>) -> R) -> R {
        STATE.with_borrow(|state| f(&state.subnets_metrics))
    }

    fn with_subnets_metrics_mut<R>(f: impl FnOnce(&mut StableSubnetsMetrics<VM>) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.subnets_metrics))
    }

    fn with_last_timestamp_per_subnet<R>(f: impl FnOnce(&StableLastTimestampPerSubnet<VM>) -> R) -> R {
        STATE.with_borrow(|state| f(&state.last_timestamp_per_subnet))
    }

    fn with_last_timestamp_per_subnet_mut<R>(f: impl FnOnce(&mut StableLastTimestampPerSubnet<VM>) -> R) -> R {
        STATE.with_borrow_mut(|state| f(&mut state.last_timestamp_per_subnet))
    }
}
