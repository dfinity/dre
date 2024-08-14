use anyhow::Context;
use candid::{CandidType, Decode, Deserialize, Encode};
use ic_agent::Agent;
use ic_base_types::{CanisterId, PrincipalId};
use ic_management_canister_types::NodeMetricsHistoryArgs;
use ic_utils::interfaces::{wallet::CallResult, WalletCanister};
use log::error;
use serde::Serialize;
use std::str::FromStr;

use crate::{CallIn, IcAgentCanisterClient};

#[derive(Clone)]
pub struct WalletCanisterWrapper {
    agent: Agent,
    management_canister: CanisterId,
}

impl From<IcAgentCanisterClient> for WalletCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self {
            agent: value.agent,
            management_canister: CanisterId::from_str("aaaaa-aa").unwrap(),
        }
    }
}

impl WalletCanisterWrapper {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            management_canister: CanisterId::from_str("aaaaa-aa").unwrap(),
        }
    }

    pub async fn get_node_metrics_history(
        &self,
        wallet_canister_id: CanisterId,
        start_at_timestamp: u64,
        subnet_id: PrincipalId,
    ) -> anyhow::Result<Vec<NodeMetricsHistoryResponse>> {
        let contract = NodeMetricsHistoryArgs {
            start_at_timestamp_nanos: start_at_timestamp,
            subnet_id,
        };
        let wallet_canister = WalletCanister::from_canister(
            ic_utils::Canister::builder()
                .with_agent(&self.agent)
                .with_canister_id(wallet_canister_id)
                .build()
                .unwrap(),
        )
        .await?;

        let callin = CallIn {
            canister: self.management_canister,
            args: Encode! { &contract }?,
            cycles: 0_u128,
            method_name: "node_metrics_history".to_string(),
        };
        let builder = if wallet_canister.version_supports_u128_cycles() {
            wallet_canister.update("wallet_call128").with_arg(&callin)
        } else {
            wallet_canister.update("wallet_call").with_arg(&callin)
        };

        let (result,): (Result<CallResult, String>,) = builder.build().call_and_wait().await.context("Failed wallet call.")?;

        match result {
            Ok(result) => match Decode!(&result.r#return, Vec<NodeMetricsHistoryResponse>) {
                Ok(result) => Ok(result.to_vec()),
                Err(err) => {
                    error!("Failed to decode Trustworthy Metrics of subnet {}.", subnet_id);
                    Err(anyhow::anyhow!(err))
                }
            },
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
}

#[derive(Default, CandidType, Deserialize, Clone, Debug, Serialize)]
pub struct NodeMetrics {
    pub node_id: PrincipalId,
    pub num_blocks_proposed_total: u64,
    pub num_block_failures_total: u64,
}

impl From<trustworthy_node_metrics_types::types::NodeMetrics> for NodeMetrics {
    fn from(value: trustworthy_node_metrics_types::types::NodeMetrics) -> Self {
        Self {
            node_id: PrincipalId::from(value.node_id),
            num_block_failures_total: value.num_blocks_failures_total,
            num_blocks_proposed_total: value.num_blocks_proposed_total,
        }
    }
}

#[derive(Default, CandidType, Deserialize, Clone, Debug, Serialize)]
pub struct NodeMetricsHistoryResponse {
    pub timestamp_nanos: u64,
    pub node_metrics: Vec<NodeMetrics>,
}
