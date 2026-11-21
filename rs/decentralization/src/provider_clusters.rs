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
            ]),
        ),
        (
            // https://forum.dfinity.org/t/enhancing-network-decentralization-proposals-for-node-provider-standards/43053/99
            "Providers that share multiple dcs 1".to_string(),
            AHashSet::from_iter([
                PrincipalId::from_str("4r6qy-tljxg-slziw-zoteo-pboxh-vlctz-hkv2d-7zior-u3pxm-mmuxb-cae").unwrap(),
                PrincipalId::from_str("ivf2y-crxj4-y6ewo-un35q-a7pum-wqmbw-pkepy-d6uew-bfmff-g5yxe-eae").unwrap(),
                PrincipalId::from_str("3oqw6-vmpk2-mlwlx-52z5x-e3p7u-fjlcw-yxc34-lf2zq-6ub2f-v63hk-lae").unwrap(),
                PrincipalId::from_str("dhywe-eouw6-hstpj-ahsnw-xnjxq-cmqks-47mrg-nnncb-3sr5d-rac6m-nae").unwrap(),
                PrincipalId::from_str("diyay-s4rfq-xnx23-zczwi-nptra-5254n-e4zn6-p7tqe-vqhzr-sd4gd-bqe").unwrap(),
            ]),
        ),
        (
            "NP Group: ZarBlue".to_string(),
            AHashSet::from_iter([
                PrincipalId::from_str("rpfvr-s3kuw-xdqrr-pvuuj-hc7hl-olytw-yxlie-fmr74-sr572-6gdqx-iqe").unwrap(),
                PrincipalId::from_str("glrjs-2dbzh-owbdd-fpp5e-eweoz-nsuto-e3jmk-tl42c-wem4f-qfpfa-qqe").unwrap(),
            ]),
        ),
    ]);
}

pub fn get_linked_providers() -> AHashMap<String, AHashSet<PrincipalId>> {
    LINKED_PROVIDERS.clone()
}
