use ic_agent::Agent;
use ic_base_types::PrincipalId;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use ic_registry_transport::pb::v1::{RegistryGetValueRequest, RegistryGetValueResponse};
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
        let request = RegistryGetValueRequest {
            key: "subnet_list".as_bytes().to_vec(),
            ..Default::default()
        };

        let mut buf = vec![];
        request.encode(&mut buf)?;

        let response = self.agent.query(&REGISTRY_CANISTER_ID.into(), "get_value").with_arg(buf).call().await?;

        let decoded_resp = RegistryGetValueResponse::decode(&response[..])?;
        if let Some(error) = decoded_resp.error {
            return Err(anyhow::anyhow!(error.reason));
        }

        let mapped = SubnetListRecord::decode(&decoded_resp.value[..])?;

        Ok(mapped.subnets.into_iter().map(|id: Vec<u8>| PrincipalId::try_from(id).unwrap()).collect())
    }
}
