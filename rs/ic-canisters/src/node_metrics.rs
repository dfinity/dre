use std::str::FromStr;

use candid::Principal;
use ic_base_types::PrincipalId;
use log::error;
use trustworthy_node_metrics_types::types::{SubnetNodeMetricsArgs, SubnetNodeMetricsResponse};

use crate::IcAgentCanisterClient;

const NODE_METRICS_CANISTER: &str = "oqi72-gaaaa-aaaam-ac2pq-cai";

pub struct NodeMetricsCanisterWrapper {
    agent: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for NodeMetricsCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        NodeMetricsCanisterWrapper::new(value)
    }
}

impl NodeMetricsCanisterWrapper {
    pub fn new(agent: IcAgentCanisterClient) -> Self {
        Self { agent }
    }

    pub async fn get_node_metrics(&self, subnet_id: Option<PrincipalId>, from_ts: Option<u64>) -> anyhow::Result<Vec<SubnetNodeMetricsResponse>> {
        self.agent
            .query::<Result<Vec<SubnetNodeMetricsResponse>, String>>(
                &Principal::from_str(NODE_METRICS_CANISTER).map_err(anyhow::Error::from)?,
                "subnet_node_metrics",
                candid::encode_one(SubnetNodeMetricsArgs {
                    ts: from_ts,
                    subnet_id: subnet_id.map(|s| s.0),
                })?,
            )
            .await?
            .map_err(|e| {
                error!("Failed to decode Node Metrics");
                anyhow::anyhow!(e)
            })
    }
}
