use crate::registry::RegistryClient;
use chrono::{DateTime, NaiveDateTime, Utc};
use ic_base_types::{NodeId, PrincipalId};
use ic_nervous_system_canisters::registry::RegistryCanister;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::{NodeRecord, NodeRewardType};
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_registry_canister_client::{RegistryDataStableMemory, StableCanisterRegistryClient, StorableRegistryKey, StorableRegistryValue};
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use rewards_calculation::rewards_calculator_results::DayUTC;
use std::cell::RefCell;
use std::sync::Arc;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static STATE: RefCell<StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>> = RefCell::new({
        let mgr = MemoryManager::init(DefaultMemoryImpl::default());
        StableBTreeMap::init(mgr.get(MemoryId::new(0)))
    });
}

pub struct DummyStore;

impl RegistryDataStableMemory for DummyStore {
    fn with_registry_map<R>(f: impl FnOnce(&StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>) -> R) -> R {
        STATE.with_borrow(f)
    }

    fn with_registry_map_mut<R>(f: impl FnOnce(&mut StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>) -> R) -> R {
        STATE.with_borrow_mut(f)
    }
}

pub fn dt_to_timestamp_nanos(datetime_str: &str) -> u64 {
    let dt = format!("{} 00:00:00", datetime_str);
    let naive = NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S").expect("Invalid date format");
    let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);
    datetime.timestamp_nanos_opt().unwrap() as u64
}

pub fn add_record_helper(key: &str, version: u64, value: Option<impl ::prost::Message>, datetime_str: &str) {
    STATE.with_borrow_mut(|map| {
        map.insert(
            StorableRegistryKey::new(key.to_string(), version, dt_to_timestamp_nanos(datetime_str)),
            StorableRegistryValue(value.map(|v| v.encode_to_vec())),
        );
    });
}

fn add_dummy_data() {
    fn generate_node_key_value(id: u64, node_type: NodeRewardType, node_operator_id: u64) -> (String, NodeRecord) {
        let value = NodeRecord {
            node_reward_type: Some(node_type as i32),
            node_operator_id: PrincipalId::new_user_test_id(node_operator_id).to_vec(),
            ..NodeRecord::default()
        };
        let key = format!("{}{}", NODE_RECORD_KEY_PREFIX, PrincipalId::new_node_test_id(id));

        (key, value)
    }
    fn generate_node_operator_key_value(id: u64, node_provider_id: u64, dc_id: String) -> (String, NodeOperatorRecord) {
        let principal_id = PrincipalId::new_user_test_id(id);
        let node_provider = PrincipalId::new_user_test_id(node_provider_id);
        let value = NodeOperatorRecord {
            node_operator_principal_id: principal_id.to_vec(),
            node_provider_principal_id: node_provider.to_vec(),
            dc_id,
            ..NodeOperatorRecord::default()
        };
        let key = format!("{}{}", NODE_OPERATOR_RECORD_KEY_PREFIX, principal_id);

        (key, value)
    }

    fn generate_dc_key_value(dc_id: String) -> (String, DataCenterRecord) {
        let value = DataCenterRecord {
            id: dc_id.clone(),
            region: "A".to_string(),
            ..DataCenterRecord::default()
        };
        let key = format!("{}{}", DATA_CENTER_KEY_PREFIX, dc_id);

        (key, value)
    }

    let dc_1_id = "X".to_string();
    let node_1_id = 1;
    let node_2_id = 2;
    let node_3_id = 3;

    let no_1_id = 10;
    let np_1_id = 20;

    let (no_1_k, no_1_v) = generate_node_operator_key_value(no_1_id, np_1_id, dc_1_id.clone());
    let (dc_1_k, dc_1_v) = generate_dc_key_value(dc_1_id);
    let (node_1_k, node_1_v) = generate_node_key_value(node_1_id, NodeRewardType::Type0, no_1_id);
    let (node_2_k, node_2_v) = generate_node_key_value(node_2_id, NodeRewardType::Type1, no_1_id);
    let (node_3_k, node_3_v) = generate_node_key_value(node_3_id, NodeRewardType::Type2, no_1_id);

    add_record_helper(&no_1_k, 39650, Some(no_1_v), "2025-07-01");
    add_record_helper(&dc_1_k, 39652, Some(dc_1_v), "2025-07-02");
    add_record_helper(&node_1_k, 39662, Some(node_1_v), "2025-07-03");
    add_record_helper(&node_2_k, 39664, Some(node_2_v), "2025-07-04");
    add_record_helper(&node_1_k, 39666, None::<NodeRecord>, "2025-07-08");
    add_record_helper(&node_3_k, 39667, Some(node_3_v), "2025-07-11");
}

fn client_for_tests() -> RegistryClient<DummyStore> {
    add_dummy_data();

    RegistryClient {
        store: StableCanisterRegistryClient::<DummyStore>::new(Arc::new(RegistryCanister::new())),
    }
}

#[test]
fn test_rewardable_nodes_deleted_nodes() {
    let client = client_for_tests();

    let from = dt_to_timestamp_nanos("2025-07-12");
    let to = dt_to_timestamp_nanos("2025-07-14");

    let mut rewardables = client.get_rewardable_nodes_per_provider(from.into(), to.into()).unwrap();

    let np_1_rewardables = rewardables.remove(&PrincipalId::new_user_test_id(20)).unwrap();

    // Verify that node_1 is not present in the rewardables
    assert!(!np_1_rewardables
        .rewardable_nodes
        .iter()
        .any(|n| n.node_id == NodeId::from(PrincipalId::new_node_test_id(1))));

    let node_2 = np_1_rewardables
        .rewardable_nodes
        .iter()
        .find(|n| n.node_id == NodeId::from(PrincipalId::new_node_test_id(2)))
        .unwrap();
    let node_3 = np_1_rewardables
        .rewardable_nodes
        .iter()
        .find(|n| n.node_id == NodeId::from(PrincipalId::new_node_test_id(3)))
        .unwrap();

    assert_eq!(node_2.rewardable_from, DayUTC::from(from));
    assert_eq!(node_2.rewardable_to, DayUTC::from(to));
    assert_eq!(node_3.rewardable_from, DayUTC::from(from));
    assert_eq!(node_3.rewardable_to, DayUTC::from(to));
}
