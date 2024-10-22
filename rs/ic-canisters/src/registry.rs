use ic_agent::Agent;
use ic_base_types::PrincipalId;
use ic_interfaces_registry::RegistryTransportRecord;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_protobuf::{
    registry::{crypto::v1::PublicKey, subnet::v1::SubnetListRecord},
    types::v1::SubnetId,
};
use ic_registry_keys::make_crypto_threshold_signing_pubkey_key;
use ic_registry_nns_data_provider::certification::decode_certified_deltas;
use ic_registry_transport::pb::v1::{
    RegistryGetChangesSinceRequest, RegistryGetChangesSinceResponse, RegistryGetLatestVersionResponse, RegistryGetValueRequest,
    RegistryGetValueResponse,
};
use ic_types::crypto::threshold_sig::ThresholdSigPublicKey;
use prost::Message;

use crate::IcAgentCanisterClient;

pub struct RegistryCanisterWrapper {
    pub agent: Agent,
}

impl From<IcAgentCanisterClient> for RegistryCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self { agent: value.agent }
    }
}

impl RegistryCanisterWrapper {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
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

    pub async fn get_certified_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryTransportRecord>> {
        let response = self.get_changes_since_inner(version, "get_certified_changes_since").await?;

        decode_certified_deltas(version, &REGISTRY_CANISTER_ID, &self.nns_public_key().await?, response.as_slice())
            .map_err(|e| anyhow::anyhow!("Error decoding certificed deltas: {:?}", e))
            .map(|(res, _, _)| res)
    }

    pub async fn get_changes_since(&self, version: u64) -> anyhow::Result<RegistryGetChangesSinceResponse> {
        let response = self.get_changes_since_inner(version, "get_changes_since").await?;

        RegistryGetChangesSinceResponse::decode(&response[..]).map_err(anyhow::Error::from)
    }

    async fn get_changes_since_inner(&self, version: u64, endpoint: &str) -> anyhow::Result<Vec<u8>> {
        let request = RegistryGetChangesSinceRequest { version };
        let mut buf = vec![];
        request.encode(&mut buf)?;

        self.agent
            .query(&REGISTRY_CANISTER_ID.into(), endpoint)
            .with_arg(buf)
            .call()
            .await
            .map_err(anyhow::Error::from)
    }

    async fn get_value(&self, request: String) -> anyhow::Result<Vec<u8>> {
        let request = RegistryGetValueRequest {
            key: request.as_bytes().to_vec(),
            ..Default::default()
        };

        let mut buf = vec![];
        request.encode(&mut buf)?;
        let response = self.agent.query(&REGISTRY_CANISTER_ID.into(), "get_value").with_arg(buf).call().await?;

        let decoded_resp = RegistryGetValueResponse::decode(&response[..])?;
        if let Some(error) = decoded_resp.error {
            return Err(anyhow::anyhow!(error.reason));
        }

        Ok(decoded_resp.value)
    }
}
