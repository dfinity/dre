use std::str::FromStr;

use crate::IcAgentCanisterClient;
use candid::Principal;
use ic_sns_wasm::pb::v1::{ListDeployedSnsesRequest, ListDeployedSnsesResponse};

const SNS_WASM_CANISTER: &str = "qaa6y-5yaaa-aaaaa-aaafa-cai";

pub struct SnsWasmCanister {
    agent: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for SnsWasmCanister {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self { agent: value }
    }
}

impl SnsWasmCanister {
    pub async fn list_deployed_snses(&self) -> anyhow::Result<ListDeployedSnsesResponse> {
        self.agent
            .query(
                &Principal::from_str(SNS_WASM_CANISTER)?,
                "list_deployed_snses",
                candid::encode_one(ListDeployedSnsesRequest {})?,
            )
            .await
    }
}
