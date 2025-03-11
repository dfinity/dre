use ahash::{AHashMap, AHashSet};
use ic_base_types::PrincipalId;
use lazy_static::lazy_static;
use std::str::FromStr;

// List of providers known to be linked. Having them grouped reduces the risk of sybil attacks.
// To add a new provider cluster, include the cluster name and its associated Principal IDs in the map.
// Always add a reference link in the comment, preferably pointing directly to the provider's explanation.
lazy_static! {
    static ref LINKED_PROVIDERS: AHashMap<String, AHashSet<PrincipalId>> = AHashMap::from_iter([
        (
            // https://forum.dfinity.org/t/sybiling-nodes-exploiting-ic-network-community-attention-required/40690/59
            "Node provider cluster 1 (6sq7t, vegae, eatbv)".to_string(),
            AHashSet::from_iter(vec![
                PrincipalId::from_str("6sq7t-knkul-fko6h-xzvnf-ktbvr-jhx7r-hapzr-kjlek-whugy-zt6ip-xqe").unwrap(),
                PrincipalId::from_str("vegae-c4chr-aetfj-7gzuh-c23sx-u2paz-vmvbn-bcage-pu7lu-mptnn-eqe").unwrap(),
                PrincipalId::from_str("eatbv-nlydd-n655c-g7j7p-gnmpz-pszdg-6e6et-veobv-ftz2y-4m752-vqe").unwrap(),
            ])
        ),
    ]);
}

pub fn get_linked_providers() -> AHashMap<String, AHashSet<PrincipalId>> {
    LINKED_PROVIDERS.clone()
}
