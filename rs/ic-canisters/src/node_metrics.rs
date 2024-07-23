use std::str::FromStr;

use candid::{Decode, Encode};
use ic_agent::Agent;
use ic_base_types::{CanisterId, PrincipalId};
use log::error;
use trustworthy_node_metrics::types::{SubnetNodeMetricsArgs, SubnetNodeMetricsResponse};

use crate::IcAgentCanisterClient;

const NODE_METRICS_CANISTER: &str = "oqi72-gaaaa-aaaam-ac2pq-cai";

pub struct NodeMetricsCanisterWrapper {
    agent: Agent,
    node_metrics_canister: CanisterId,
}

impl From<IcAgentCanisterClient> for NodeMetricsCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        NodeMetricsCanisterWrapper::new(value.agent)
    }
}

impl NodeMetricsCanisterWrapper {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            node_metrics_canister: CanisterId::from_str(NODE_METRICS_CANISTER).unwrap(),
        }
    }

    pub async fn get_node_metrics(&self, subnet_id: Option<PrincipalId>, from_ts: Option<u64>) -> anyhow::Result<Vec<SubnetNodeMetricsResponse>> {
        let request = SubnetNodeMetricsArgs {
            ts: from_ts,
            subnet_id: subnet_id.map(|s| s.0),
        };

        let response = self
            .agent
            .query(&self.node_metrics_canister.into(), "subnet_node_metrics")
            .with_arg(Encode!(&request)?)
            .call()
            .await?;

        match Decode!(&response, Result<Vec<SubnetNodeMetricsResponse>, String>)? {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("Failed to decode Node Metrics");
                Err(anyhow::anyhow!(err))
            }
        }
    }
}
