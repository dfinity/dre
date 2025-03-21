use crate::canister_client::ICCanisterClient;
use crate::metrics::MetricsManager;
use crate::registry_store::{RegistryStoreData, StableLocalRegistry};
use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use std::cell::RefCell;
use std::rc::Rc;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

const LOCAL_REGISTRY_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static METRICS_MANAGER: Rc<Rc<RefCell<MetricsManager<VM>>>> = {
        let metrics_manager = MetricsManager {
            client: Box::new(ICCanisterClient),
            subnets_to_retry: stable_btreemap_init(SUBNETS_TO_RETRY_MEMORY_ID),
            subnets_metrics: stable_btreemap_init(SUBNETS_METRICS_MEMORY_ID),
            last_timestamp_per_subnet: stable_btreemap_init(LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID),
        };

        Rc::new(Rc::new(RefCell::new(metrics_manager)))
    };

    pub static STATE: RefCell<State> = RefCell::new(State::init());

}

pub fn stable_btreemap_init<K: Storable + Clone + Ord, V: Storable>(memory_id: MemoryId) -> StableBTreeMap<K, V, VM> {
    with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(memory_id)))
}
fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}

pub struct State {
    local_registry: StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>,
}

impl State {
    fn init() -> Self {
        State {
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
