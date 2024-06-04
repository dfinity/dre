use std::str::FromStr;

use crate::IcAgentCanisterClient;
use ic_agent::Agent;
use ic_base_types::CanisterId;
use ic_sns_wasm::pb::v1::{ListDeployedSnsesRequest, ListDeployedSnsesResponse};

const SNS_WASM_CANISTER: &str = "qaa6y-5yaaa-aaaaa-aaafa-cai";

#[derive(Clone)]
pub struct SnsWasmCanister {
    agent: Agent,
    sns_wasm_canister: CanisterId,
}

impl From<IcAgentCanisterClient> for SnsWasmCanister {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self {
            agent: value.agent,
            sns_wasm_canister: CanisterId::from_str(SNS_WASM_CANISTER).unwrap(),
        }
    }
}

impl SnsWasmCanister {
    pub fn new(agent: Agent, sns_canister_id: Option<&str>) -> Self {
        Self {
            agent,
            sns_wasm_canister: CanisterId::from_str(sns_canister_id.unwrap_or(SNS_WASM_CANISTER)).unwrap(),
        }
    }

    pub async fn list_deployed_snses(&self) -> anyhow::Result<ListDeployedSnsesResponse> {
        let arg = candid::encode_one(ListDeployedSnsesRequest {})?;
        let response = self
            .agent
            .query(&self.sns_wasm_canister.into(), "list_deployed_snses")
            .with_arg(arg)
            .call()
            .await?;
        Ok(candid::decode_one(&response)?)
    }
}
