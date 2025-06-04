use ic_base_types::PrincipalId;
use ic_interfaces_registry::RegistryRecord;
use ic_protobuf::{
    registry::{crypto::v1::PublicKey, subnet::v1::SubnetListRecord},
    types::v1::SubnetId,
};
use ic_registry_keys::make_crypto_threshold_signing_pubkey_key;
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_types::crypto::threshold_sig::ThresholdSigPublicKey;
use prost::Message;
use url::Url;

pub struct RegistryCanisterWrapper {
    registry_canister: RegistryCanister,
}

impl RegistryCanisterWrapper {
    pub fn new(urls: Vec<Url>) -> Self {
        Self {
            registry_canister: RegistryCanister::new(urls),
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
        self.registry_canister.get_latest_version().await.map_err(anyhow::Error::from)
    }

    pub async fn get_certified_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryRecord>> {
        self.registry_canister
            .get_certified_changes_since(version, &self.nns_public_key().await?)
            .await
            .map_err(|e| anyhow::anyhow!("Error decoding certificed deltas: {:?}", e))
            .map(|(res, _, _)| res)
    }

    async fn get_value(&self, request: String) -> anyhow::Result<Vec<u8>> {
        let (response, _) = self.registry_canister.get_value(request.as_bytes().to_vec(), None).await?;
        Ok(response)
    }
}
