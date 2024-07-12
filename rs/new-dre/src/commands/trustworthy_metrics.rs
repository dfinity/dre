use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::Args;
use ic_canisters::{management::WalletCanisterWrapper, registry::RegistryCanisterWrapper};
use ic_types::{CanisterId, PrincipalId};
use log::{info, warn};

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct TrustworthyMetrics {
    /// Wallet that should be used to query node metrics history
    /// in form of canister id
    pub wallet: String,

    /// Start at timestamp in nanoseconds
    pub start_at_timestamp: u64,

    /// Vector of subnets to query, if empty will dump metrics for
    /// all subnets
    pub subnet_ids: Vec<PrincipalId>,
}

impl ExecutableCommand for TrustworthyMetrics {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let lock = Mutex::new(());
        let canister_agent = ctx.create_ic_agent_canister_client(Some(lock))?;
        info!("Started action...");
        let wallet_client = Arc::new(WalletCanisterWrapper::new(canister_agent.agent.clone()));

        let subnets = match &self.subnet_ids.is_empty() {
            false => self.subnet_ids.clone(),
            true => {
                let registry_client = RegistryCanisterWrapper::new(canister_agent.agent);
                registry_client.get_subnets().await?
            }
        };

        let mut metrics_by_subnet = BTreeMap::new();
        info!("Running in parallel mode");

        let wallet: CanisterId = CanisterId::from_str(&self.wallet)?;

        let handles = subnets
            .into_iter()
            .map(|s| (s, wallet_client.clone(), self.start_at_timestamp))
            .map(|(s, w, start)| {
                info!("Spawning thread for subnet: {}", s);
                tokio::spawn(async move { (s, w.get_node_metrics_history(wallet, start, s).await) })
            });

        for handle in handles {
            let (subnet, maybe_metrics) = handle.await?;
            match maybe_metrics {
                Ok(m) => {
                    info!("Received metrics for subnet: {}", subnet);
                    metrics_by_subnet.insert(subnet, m);
                }
                Err(e) => {
                    warn!("Couldn't fetch trustworthy metrics for subnet {}: {}", subnet, e);
                }
            };
        }

        println!("{}", serde_json::to_string_pretty(&metrics_by_subnet)?);

        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
