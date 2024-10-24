use candid::Principal;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};

pub(crate) async fn update_registry() {
    let response = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_certified_changes_since", serialize_get_changes_since_request(40000).unwrap(), 0)

    .await
    .unwrap();

    let (delta, version) = deserialize_get_changes_since_response(response).unwrap();


    ic_cdk::println!("latest version: {}", version)

}
