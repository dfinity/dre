use ic_base_types::RegistryVersion;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;

// This value is set as 2 times the max key size present in the registry
const MAX_REGISTRY_KEY_SIZE: u32 = 200;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
pub struct StorableRegistryKey {
    pub key: String,
    pub version: RegistryVersion,
}
impl StorableRegistryKey {
    pub fn new(key: String, version: RegistryVersion) -> Self {
        Self { key, version }
    }
}

impl Storable for StorableRegistryKey {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut storable_key = vec![];
        let version_b = self.version.get().to_be_bytes().to_vec();
        let key_b = self.key.as_bytes().to_vec();

        storable_key.extend_from_slice(&version_b);
        storable_key.extend_from_slice(&key_b);

        Cow::Owned(storable_key)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let (version_bytes, key_bytes) = bytes.split_at(8);

        let version_u64 = u64::from_be_bytes(version_bytes.try_into().expect("Invalid version bytes"));
        let version = RegistryVersion::new(version_u64);
        let key = String::from_utf8(key_bytes.to_vec()).expect("Invalid UTF-8 in key");
        Self { key, version }
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_REGISTRY_KEY_SIZE + size_of::<u64>() as u32,
        is_fixed_size: false,
    };
}

#[derive(Clone, Debug)]
pub struct StorableRegistryValue(pub Option<Vec<u8>>);

impl Storable for StorableRegistryValue {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(Option::from_bytes(bytes))
    }

    const BOUND: Bound = Bound::Unbounded;
}
