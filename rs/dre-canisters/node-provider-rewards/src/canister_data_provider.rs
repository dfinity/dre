use crate::data_provider::{StableMemoryBorrower, StorableRegistryKey, StorableRegistryValue};
use crate::stable_memory;
use ic_stable_structures::memory_manager::VirtualMemory;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};

#[derive(Default)]
pub struct StableMemoryStore;
impl StableMemoryBorrower for StableMemoryStore {
    fn with_borrow<R>(f: impl FnOnce(&StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VirtualMemory<DefaultMemoryImpl>>) -> R) -> R {
        stable_memory::REGISTRY.with_borrow(|registry_stored| f(registry_stored))
    }

    fn with_borrow_mut<R>(
        f: impl FnOnce(&mut StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VirtualMemory<DefaultMemoryImpl>>) -> R,
    ) -> R {
        stable_memory::REGISTRY.with_borrow_mut(|registry_stored| f(registry_stored))
    }
}
