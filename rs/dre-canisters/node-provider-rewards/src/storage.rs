use crate::canister_client::ICCanisterClient;
use crate::metrics::MetricsManager;
use crate::registry_store::{RegistryStoreData, StableLocalRegistry};
use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use std::cell::RefCell;
use std::rc::Rc;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static STATE: RefCell<State> = RefCell::new(State::init());
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}
pub fn stable_btreemap_init<K: Storable + Clone + Ord, V: Storable>(memory_id: MemoryId) -> StableBTreeMap<K, V, VM> {
    with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(memory_id)))
}

const LOCAL_REGISTRY_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

pub struct State {
    metrics_manager: Rc<RefCell<MetricsManager<VM>>>,
    local_registry: StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>,
}

impl State {
    fn init() -> Self {
        State {
            metrics_manager: Rc::new(RefCell::new(MetricsManager {
                client: Rc::new(ICCanisterClient),
                subnets_to_retry: stable_btreemap_init(SUBNETS_TO_RETRY_MEMORY_ID),
                subnets_metrics: stable_btreemap_init(SUBNETS_METRICS_MEMORY_ID),
                last_timestamp_per_subnet: stable_btreemap_init(LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID),
            })),
            local_registry: stable_btreemap_init(LOCAL_REGISTRY_MEMORY_ID),
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

pub(crate) fn metrics_manager_rc() -> Rc<RefCell<MetricsManager<VM>>> {
    STATE.with_borrow_mut(|state| state.metrics_manager.clone())
}
