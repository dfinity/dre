use std::collections::BTreeMap;

use ic_crypto_tree_hash::{LabeledTree, MixedHashTree};
use ic_interfaces_registry::RegistryTransportRecord;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::{registry_mutation::Type, CertifiedResponse, RegistryAtomicMutateRequest};
use ic_types::RegistryVersion;
use prost::Message;
use serde::Deserialize;
use tree_deserializer::{types::Leb128EncodedU64, LabeledTreeDeserializer};

#[derive(Deserialize)]
struct CertifiedPayload {
    current_version: Leb128EncodedU64,
    #[serde(default)]
    delta: BTreeMap<u64, Protobuf<RegistryAtomicMutateRequest>>,
}

struct Protobuf<T>(T);

impl<'de, T> serde::Deserialize<'de> for Protobuf<T>
where
    T: prost::Message + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use std::fmt;
        use std::marker::PhantomData;

        struct ProtobufVisitor<T: prost::Message>(PhantomData<T>);

        impl<'de, T: prost::Message + Default> serde::de::Visitor<'de> for ProtobufVisitor<T> {
            type Value = Protobuf<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "Protobuf message of type {}", std::any::type_name::<T>())
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::decode(v).map(Protobuf).map_err(E::custom)
            }
        }

        let visitor: ProtobufVisitor<T> = ProtobufVisitor(PhantomData);
        deserializer.deserialize_bytes(visitor)
    }
}

pub fn decode_hash_tree(since_version: u64, hash_tree: MixedHashTree) -> anyhow::Result<Vec<RegistryTransportRecord>> {
    // Extract structured deltas from their tree representation.
    let labeled_tree = LabeledTree::<Vec<u8>>::try_from(hash_tree).expect("failed tree");
    let certified_payload = CertifiedPayload::deserialize(LabeledTreeDeserializer::new(&labeled_tree))?;

    let changes = certified_payload
        .delta
        .into_iter()
        .flat_map(|(v, mutate_req)| {
            mutate_req.0.mutations.into_iter().map(move |m| {
                let value = if m.mutation_type == Type::Delete as i32 { None } else { Some(m.value) };
                RegistryTransportRecord {
                    key: String::from_utf8_lossy(&m.key[..]).to_string(),
                    value,
                    version: RegistryVersion::from(v),
                }
            })
        })
        .collect();

    Ok(changes)
}

pub struct RegistryClient();

impl RegistryClient {
    pub async fn new() -> Self {
        let certified_response: CertifiedResponse =
            ic_cdk::api::call::call::<_, CertifiedResponse>(REGISTRY_CANISTER_ID, "get_certified_latest_version", ())
                .await
                .map_err(|(code, msg)| ic_cdk::println!("ERROR Pietro: {}, {}", code.unwrap_or_default(), msg))
                .unwrap();

        let hash_tree: MixedHashTree = certified_response
            .hash_tree
            .expect("no hash tree in a certified response")
            .try_into()
            .expect("failed to decode hash tree from protobuf");

        let hash_tree = decode_hash_tree(0, hash_tree).unwrap();

        for record in registry_records.iter().take(10) {
            ic_cdk::println!("records: {}", record.key)
        }

        Self()
    }
}
