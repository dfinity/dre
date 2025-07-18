use crate::metrics::{ICCanisterClient, MetricsManager};
use crate::registry::RegistryClient;
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_registry_canister_client::{RegistryDataStableMemory, StableCanisterRegistryClient, StorableRegistryKey, StorableRegistryValue};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

const REGISTRY_STORE_MEMORY_ID: MemoryId = MemoryId::new(0);
const SUBNETS_METRICS_MEMORY_ID: MemoryId = MemoryId::new(1);
const LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID: MemoryId = MemoryId::new(2);
const SUBNETS_TO_RETRY_MEMORY_ID: MemoryId = MemoryId::new(3);

pub fn stable_btreemap_init<K: Storable + Clone + Ord, V: Storable>(memory_id: MemoryId) -> StableBTreeMap<K, V, VM> {
    with_memory_manager(|mgr| StableBTreeMap::init(mgr.get(memory_id)))
}
fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|memory_manager| f(&memory_manager.borrow()))
}

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static METRICS_MANAGER: Rc<MetricsManager<VM>> = {
        let metrics_manager = MetricsManager {
            client: Box::new(ICCanisterClient),
            subnets_to_retry: RefCell::new(stable_btreemap_init(SUBNETS_TO_RETRY_MEMORY_ID)),
            subnets_metrics: RefCell::new(stable_btreemap_init(SUBNETS_METRICS_MEMORY_ID)),
            last_timestamp_per_subnet: RefCell::new(stable_btreemap_init(LAST_TIMESTAMP_PER_SUBNET_MEMORY_ID)),
        };

        Rc::new(metrics_manager)
    };

    static REGISTRY_DATA_STORE_BTREE_MAP: RefCell<StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>>
        = RefCell::new(stable_btreemap_init(REGISTRY_STORE_MEMORY_ID));

    pub static REGISTRY_STORE: Rc<RegistryClient<RegistryStoreStableMemoryBorrower>> = {
        let registry_client = RegistryClient {
            store: StableCanisterRegistryClient::<RegistryStoreStableMemoryBorrower>::new(
            Arc::new(RegistryCanister::new()))
        };
        Rc::new(registry_client)
    };

}

pub struct RegistryStoreStableMemoryBorrower;

impl RegistryDataStableMemory for RegistryStoreStableMemoryBorrower {
    fn with_registry_map<R>(f: impl FnOnce(&StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>) -> R) -> R {
        REGISTRY_DATA_STORE_BTREE_MAP.with_borrow(f)
    }

    fn with_registry_map_mut<R>(
        f: impl FnOnce(&mut StableBTreeMap<ic_registry_canister_client::StorableRegistryKey, ic_registry_canister_client::StorableRegistryValue, VM>) -> R,
    ) -> R {
        REGISTRY_DATA_STORE_BTREE_MAP.with_borrow_mut(f)
    }
}
