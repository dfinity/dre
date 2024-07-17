use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Mutex,
};

use clap::Args;
use futures::future::try_join_all;
use ic_canisters::{
    management::{NodeMetrics, NodeMetricsHistoryResponse},
    node_metrics::NodeMetricsCanisterWrapper,
};
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Debug, Args)]
pub struct FromNodeMetrics {
    /// Start at timestamp in nanoseconds, if empty will dump daily metrics
    /// since May 18, 2024
    pub start_at_timestamp: u64,

    /// Vector of subnets to query, if empty will dump metrics for
    /// all subnets
    pub subnet_ids: Vec<PrincipalId>,
}

impl ExecutableCommand for FromNodeMetrics {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let lock = Mutex::new(());
        let mut metrics_by_subnet = BTreeMap::new();

        let canister_agent = ctx.create_ic_agent_canister_client(Some(lock))?;
        info!("Started action...");

        let metrics_client = NodeMetricsCanisterWrapper::new(canister_agent.agent);

        let node_metrics_response = match &self.subnet_ids.is_empty() {
            true => metrics_client.get_node_metrics(None, Some(self.start_at_timestamp)).await?,
            false => {
                let subnets = self.subnet_ids.clone();
                let metrics_client_ref = &metrics_client;

                try_join_all(
                    subnets
                        .into_iter()
                        .map(|subnet| async move { metrics_client_ref.get_node_metrics(Some(subnet), Some(self.start_at_timestamp)).await }),
                )
                .await?
                .into_iter()
                .flatten()
                .collect_vec()
            }
        };

        for metrics in node_metrics_response {
            let node_metrics_history = NodeMetricsHistoryResponse {
                timestamp_nanos: metrics.ts,
                node_metrics: metrics
                    .node_metrics
                    .into_iter()
                    .map(|m| NodeMetrics {
                        node_id: PrincipalId::from(m.node_id),
                        num_block_failures_total: m.num_block_failures_total,
                        num_blocks_proposed_total: m.num_blocks_proposed_total,
                    })
                    .collect_vec(),
            };

            match metrics_by_subnet.entry(metrics.subnet_id) {
                Entry::Occupied(mut entry) => {
                    let v: &mut Vec<NodeMetricsHistoryResponse> = entry.get_mut();
                    v.push(node_metrics_history)
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![node_metrics_history]);
                }
            }
        }

        metrics_by_subnet.values_mut().for_each(|f| f.sort_by_key(|k| k.timestamp_nanos));

        println!("{}", serde_json::to_string_pretty(&metrics_by_subnet)?);

        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
