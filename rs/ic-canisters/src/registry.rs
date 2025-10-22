use crate::{CanisterVersion, IcAgentCanisterClient};
use candid::{Decode, Encode};
use ic_agent::Agent;
use ic_base_types::PrincipalId;
use ic_interfaces_registry::RegistryRecord;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_protobuf::{
    registry::{crypto::v1::PublicKey, subnet::v1::SubnetListRecord},
    types::v1::SubnetId,
};
use ic_registry_canister_api::{Chunk, GetChunkRequest};
use ic_registry_keys::make_crypto_threshold_signing_pubkey_key;
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_registry_transport::{
    dechunkify_get_value_response_content,
    pb::v1::{HighCapacityRegistryGetValueResponse, RegistryGetLatestVersionResponse, RegistryGetValueRequest},
    GetChunk,
};
use ic_types::crypto::threshold_sig::ThresholdSigPublicKey;
use prost::Message;
use url::Url;

pub struct RegistryCanisterWrapper {
    pub agent: Agent,
    ic_wrapper: RegistryCanister,
}

impl From<IcAgentCanisterClient> for RegistryCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self {
            agent: value.agent,
            ic_wrapper: RegistryCanister::new(vec![value.nns_url]),
        }
    }
}

pub async fn registry_canister_version(url: Url) -> anyhow::Result<CanisterVersion> {
    super::canister_version(url, REGISTRY_CANISTER_ID.into()).await
}

impl RegistryCanisterWrapper {
    pub fn new(agent: Agent, nns_url: Url) -> Self {
        Self {
            agent,
            ic_wrapper: RegistryCanister::new(vec![nns_url]),
        }
    }

    pub async fn get_subnets(&self) -> anyhow::Result<Vec<PrincipalId>> {
        let decoded_resp = self.get_value("subnet_list".to_string()).await?;

        let mapped = SubnetListRecord::decode(decoded_resp.as_slice())?;

        Ok(mapped.subnets.into_iter().map(|id: Vec<u8>| PrincipalId::try_from(id).unwrap()).collect())
    }

    pub async fn nns_subnet_id(&self) -> anyhow::Result<SubnetId> {
        let decoded_resp = self.get_value("nns_subnet_id".to_string()).await?;

        SubnetId::decode(decoded_resp.as_slice()).map_err(anyhow::Error::from)
    }

    pub async fn nns_public_key(&self) -> anyhow::Result<ThresholdSigPublicKey> {
        let subnet_id = self.nns_subnet_id().await?;
        let subnet_id = ic_types::SubnetId::new(ic_types::PrincipalId::try_from(
            subnet_id.principal_id.ok_or(anyhow::anyhow!("Failed to find nns subnet id"))?.raw,
        )?);

        let decoded_resp = self.get_value(make_crypto_threshold_signing_pubkey_key(subnet_id)).await?;

        ThresholdSigPublicKey::try_from(PublicKey::decode(decoded_resp.as_slice())?).map_err(anyhow::Error::from)
    }

    pub async fn get_latest_version(&self) -> anyhow::Result<u64> {
        let response = self.agent.query(&REGISTRY_CANISTER_ID.into(), "get_latest_version").call().await?;

        RegistryGetLatestVersionResponse::decode(response.as_slice())
            .map_err(anyhow::Error::from)
            .map(|r| r.version)
    }

    pub async fn get_certified_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryRecord>> {
        self.ic_wrapper
            .get_certified_changes_since(version, &self.nns_public_key().await?)
            .await
            .map_err(|e| anyhow::anyhow!("Error decoding certificed deltas: {:?}", e))
            .map(|(res, _, _)| res)
    }

    async fn get_value(&self, request: String) -> anyhow::Result<Vec<u8>> {
        let request = RegistryGetValueRequest {
            key: request.as_bytes().to_vec(),
            ..Default::default()
        };

        let mut buf = vec![];
        request.encode(&mut buf)?;
        let response = self.agent.query(&REGISTRY_CANISTER_ID.into(), "get_value").with_arg(buf).call().await?;

        let decoded_resp = HighCapacityRegistryGetValueResponse::decode(&response[..])?;
        if let Some(error) = decoded_resp.error {
            return Err(anyhow::anyhow!(error.reason));
        }
        let Some(content) = decoded_resp.content else {
            return Err(anyhow::anyhow!("No content found in get_value response"));
        };
        let dechunkified = dechunkify_get_value_response_content(
            content,
            self,
        )
        .await?;

        Ok(dechunkified)
    }
}

#[async_trait::async_trait]
impl GetChunk for RegistryCanisterWrapper {
    /// Just calls the Registry canister's get_chunk method.
    async fn get_chunk_without_validation(&self, content_sha256: &[u8]) -> Result<Vec<u8>, String> {
        fn new_err(cause: impl std::fmt::Debug) -> String {
            format!("Unable to fetch large registry record: {cause:?}",)
        }

        // Call get_chunk.
        let content_sha256 = Some(content_sha256.to_vec());
        let request = Encode!(&GetChunkRequest { content_sha256 }).map_err(new_err)?;
        let get_chunk_response: Vec<u8> = self
            .agent
            .query(&REGISTRY_CANISTER_ID.into(), "get_chunk")
            .with_arg(request)
            .call()
            .await
            .map_err(new_err)?;

        // Extract chunk from get_chunk call.
        let Chunk { content } = Decode!(&get_chunk_response, Result<Chunk, String>)
            .map_err(new_err)? // unable to decode
            .map_err(new_err)?; // Registry canister returned Err.
        content.ok_or_else(|| new_err("content in get_chunk response is null (not even an empty string)"))
    }
}
