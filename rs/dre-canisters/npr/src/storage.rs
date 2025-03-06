use crate::metrics::{MetricsManager, SubnetIdStored};
use crate::registry::RegistryClient;
use crate::registry_store::CanisterRegistryStore;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableVec};
use std::cell::RefCell;
use std::rc::Rc;

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
    registry_store: Rc<RefCell<CanisterRegistryStore<VM>>>,

    registry_client: RegistryClient<VM>,

    metrics_manager: Rc<RefCell<MetricsManager<VM>>>,
}

impl Default for State {
    fn default() -> Self {
        todo!()
    }
}

impl State {
    fn new() -> Self {
        let registry_store =
            with_memory_manager(|memory_manager| Rc::new(RefCell::new(CanisterRegistryStore::init(memory_manager.get(REGISTRY_STORE_MEMORY_ID)))));

        let registry_client = RegistryClient::init(Rc::clone(&registry_store));
        let metrics_manager = with_memory_manager(|memory_manager| {
            let subnets_to_retry_stored: StableVec<SubnetIdStored, _> =
                StableVec::init(memory_manager.get(METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID)).expect("Failed to initialize retry subnets");
            let subnets_to_retry = subnets_to_retry_stored.iter().map(|subnet_id| subnet_id.0).collect();

            MetricsManager::init(
                memory_manager.get(METRICS_MANAGER_SUBNETS_METRICS_MEMORY_ID),
                memory_manager.get(METRICS_MANAGER_LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID),
                subnets_to_retry,
            )
        });

        Self {
            registry_store,
            registry_client,
            metrics_manager: Rc::new(RefCell::new(metrics_manager)),
        }
    }
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}

pub fn with_registry_client<R>(f: impl FnOnce(&RegistryClient<VM>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.registry_client))
}

pub fn with_registry_store<R>(f: impl FnOnce(&Rc<RefCell<CanisterRegistryStore<VM>>>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.registry_store))
}

pub fn with_metrics_manager<R>(f: impl FnOnce(&Rc<RefCell<MetricsManager<VM>>>) -> R) -> R {
    STATE.with_borrow(|state| f(&state.metrics_manager))
}

pub fn reset_stable_memory() {
    MEMORY_MANAGER.with(|mm| *mm.borrow_mut() = MemoryManager::init(DefaultMemoryImpl::default()));
    STATE.with(|cell| *cell.borrow_mut() = State::new());
}

pub fn pre_upgrade() {
    with_memory_manager(|memory_manager| {
        let memory = memory_manager.get(METRICS_MANAGER_SUBNETS_TO_RETRY_MEMORY_ID);
        let subnets_to_retry_stored: StableVec<SubnetIdStored, _> = StableVec::new(memory).expect("Failed to initialize retry subnets");

        with_metrics_manager(|metrics_manager| {
            for subnet_id in metrics_manager.borrow().subnets_to_retry.iter() {
                subnets_to_retry_stored
                    .push(&SubnetIdStored(*subnet_id))
                    .expect("Failed to push subnet id");
            }
        })
    })
}
