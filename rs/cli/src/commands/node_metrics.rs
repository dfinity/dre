use std::{
    collections::{btree_map::Entry, BTreeMap},
    str::FromStr,
    sync::Arc,
};

use clap::{error::ErrorKind, Args};
use ic_canisters::{
    management::{NodeMetricsHistoryResponse, WalletCanisterWrapper},
    node_metrics::NodeMetricsCanisterWrapper,
    registry::RegistryCanisterWrapper,
};
use ic_types::{CanisterId, PrincipalId};
use itertools::Itertools;
use log::{info, warn};

use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};

type CLINodeMetrics = BTreeMap<PrincipalId, Vec<NodeMetricsHistoryResponse>>;

#[derive(Args, Debug)]
pub struct NodeMetrics {
    /// If specified trustworthy node metrics history will be fetched from the IC.
    /// If not untrusted node metrics will be fetched from node metrics canister
    #[clap(long, global = true)]
    pub trustworthy: bool,

    /// Wallet that should be used to query trustworthy node metrics history
    /// in form of canister id
    #[clap(long)]
    pub wallet: Option<String>,

    /// Start at timestamp in nanoseconds
    pub start_at_timestamp: u64,

    /// Vector of subnets to query, if empty will dump metrics for
    /// all subnets
    pub subnet_ids: Vec<PrincipalId>,
}

impl NodeMetrics {
    async fn get_trustworthy_metrics(&self, canister_agent: ic_canisters::IcAgentCanisterClient) -> anyhow::Result<CLINodeMetrics> {
        let mut metrics_by_subnet = BTreeMap::new();
        let wallet: CanisterId = CanisterId::from_str(self.wallet.as_ref().unwrap().as_str())?;
        let wallet_client = Arc::new(WalletCanisterWrapper::new(canister_agent.agent.clone()));

        let subnets = match &self.subnet_ids.is_empty() {
            false => self.subnet_ids.clone(),
            true => {
                let registry_client = RegistryCanisterWrapper::new(canister_agent.agent, canister_agent.nns_url);
                registry_client.get_subnets().await?
            }
        };

        let handles = subnets
            .into_iter()
            .map(|s| (s, wallet_client.clone(), self.start_at_timestamp))
            .map(|(s, w, start)| {
                info!("Spawning thread for subnet: {}", s);
                tokio::spawn(async move { (s, w.get_node_metrics_history(wallet, start, s).await) })
            });

        for (subnet, maybe_metrics) in futures::future::try_join_all(handles).await? {
            match maybe_metrics {
                Result::Ok(m) => {
                    info!("Received metrics for subnet: {}", subnet);
                    metrics_by_subnet.insert(subnet, m);
                }
                Err(e) => {
                    warn!("Couldn't fetch trustworthy metrics for subnets {}: {}", subnet, e);
                }
            };
        }

        Ok(metrics_by_subnet)
    }

    async fn get_untrusted_metrics(&self, canister_agent: ic_canisters::IcAgentCanisterClient) -> anyhow::Result<CLINodeMetrics> {
        let mut metrics_by_subnet = BTreeMap::new();
        let metrics_client: NodeMetricsCanisterWrapper = canister_agent.into();

        let node_metrics_response = match &self.subnet_ids.is_empty() {
            true => metrics_client.get_node_metrics(None, Some(self.start_at_timestamp)).await?,
            false => {
                let subnets = self.subnet_ids.clone();
                let metrics_client_ref = &metrics_client;

                futures::future::try_join_all(
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
            let subnet = PrincipalId::from(metrics.subnet_id);

            let management_metrics = metrics.node_metrics.into_iter().map(|m| m.into()).collect_vec();

            let management_metrics_history = NodeMetricsHistoryResponse {
                timestamp_nanos: metrics.ts,
                node_metrics: management_metrics,
            };

            match metrics_by_subnet.entry(subnet) {
                Entry::Occupied(mut entry) => {
                    let v: &mut Vec<NodeMetricsHistoryResponse> = entry.get_mut();
                    v.push(management_metrics_history)
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![management_metrics_history]);
                }
            }
        }
        metrics_by_subnet.values_mut().for_each(|f| f.sort_by_key(|k| k.timestamp_nanos));

        Ok(metrics_by_subnet)
    }
}

impl ExecutableCommand for NodeMetrics {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        info!("Started action...");

        let metrics_by_subnet = if self.trustworthy {
            self.get_trustworthy_metrics(canister_agent).await
        } else {
            self.get_untrusted_metrics(canister_agent).await
        }?;

        println!("{}", serde_json::to_string_pretty(&metrics_by_subnet)?);

        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        if self.trustworthy && self.wallet.is_none() {
            cmd.error(ErrorKind::MissingRequiredArgument, "Wallet is required for fetching trustworthy metrics.")
                .exit();
        }
    }
}
