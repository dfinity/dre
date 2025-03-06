use crate::metrics::{MetricsManager, SubnetIdStored};
use crate::registry::RegistryClient;
use crate::registry_store::CanisterRegistryStore;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, Memory, StableLog, StableVec};
use std::cell::RefCell;
use std::future::Future;
use std::io::Write;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread::LocalKey;

const REGISTRY_STORE_MEMORY_ID: MemoryId = MemoryId::new(0);

const METRICS_MANAGER_SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const METRICS_MANAGER_LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static STATE: RefCell<State> = RefCell::new(State::new());
}

pub struct State {
    // Events for audit purposes.
    registry_store: Arc<RwLock<CanisterRegistryStore<VM>>>,

    // Neurons stored in stable storage.
    registry_client: RegistryClient<VM>,

    // Neuron indexes stored in stable storage.
    metrics_manager: Arc<RwLock<MetricsManager<VM>>>,
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}

impl State {
    fn new() -> Self {
        let registry_store =
            with_memory_manager(|memory_manager| Arc::new(RwLock::new(CanisterRegistryStore::init(memory_manager.get(REGISTRY_STORE_MEMORY_ID)))));

        let registry_client = RegistryClient::init(Arc::clone(&registry_store));
        let metrics_manager = Arc::new(RwLock::new(with_memory_manager(|memory_manager| {
            let subnets_to_retry_stored: StableVec<SubnetIdStored, _> =
                StableVec::init(memory_manager.get(METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID)).expect("Failed to initialize retry subnets");
            let subnets_to_retry = subnets_to_retry_stored.iter().map(|subnet_id| subnet_id.0).collect();

            MetricsManager::init(
                memory_manager.get(METRICS_MANAGER_SUBNETS_METRICS_MEMORY_ID),
                memory_manager.get(METRICS_MANAGER_LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID),
                subnets_to_retry,
            )
        })));

        Self {
            registry_store,
            registry_client,
            metrics_manager,
        }
    }
}

pub fn with_registry_client<R>(f: impl FnOnce(&RegistryClient<VM>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.registry_client))
}

pub fn with_metrics_manager<R>(f: impl FnOnce(&Arc<RwLock<MetricsManager<VM>>>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.metrics_manager))
}

pub fn with_registry_store<R>(f: impl FnOnce(&Arc<RwLock<CanisterRegistryStore<VM>>>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.registry_store))
}
