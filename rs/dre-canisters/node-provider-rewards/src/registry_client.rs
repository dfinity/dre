use dfn_core::{api::call_with_cleanup, call};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};
use on_wire::bytes;

pub(crate) async fn update_registry() {
    let response = call(
        REGISTRY_CANISTER_ID,
        "get_changes_since",
        bytes,
        serialize_get_changes_since_request(41000).unwrap(),
    )
    .await
    .unwrap();

    let (delta, version) = deserialize_get_changes_since_response(response).unwrap();


    ic_cdk::println!("latest version: {}", version)

}





// pub fn make_crypto_threshold_signing_pubkey_key(subnet_id: SubnetId) -> String {
//     format!("{}{}", "crypto_threshold_signing_public_key_", subnet_id)
// }

// pub(crate) async fn update() {
//     let response = call(
//         REGISTRY_CANISTER_ID,
//         "get_certified_changes_since",
//         bytes,
//         serialize_get_changes_since_request(0).unwrap(),
//     )
//     .await
//     .unwrap();

//     let dese = deserialize_get_changes_since_response(response).unwrap();

//     let c = decode_certified_deltas(0, &REGISTRY_CANISTER_ID, &nns_public_key().await.unwrap(), response.as_slice())
//     .map_err(|e| anyhow::anyhow!("Error decoding certificed deltas: {:?}", e))
//     .map(|(res, _, _)| res);

// }


// async fn nns_public_key() -> anyhow::Result<ThresholdSigPublicKey> {
 // let (nns_subnet_id, _): (SubnetId, _) = ic_nns_common::registry::get_value(ROOT_SUBNET_ID_KEY.as_bytes(), None).await?;

//     let key = make_crypto_threshold_signing_pubkey_key(SubnetId::new(PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap()))
//     .as_bytes();

//     let current_result: Vec<u8> = call(
//         REGISTRY_CANISTER_ID,
//         "get_value",
//         bytes,
//         serialize_get_value_request(key.to_vec(), version).unwrap(),
//     )
//     .await
//     .unwrap();
//     Ok(
//         ThresholdSigPublicKey::try_from(PublicKey::decode(current_result.as_slice()).expect("invalid public key"))
//             .expect("failed to create thresholdsig public key"),
//     )
// }

