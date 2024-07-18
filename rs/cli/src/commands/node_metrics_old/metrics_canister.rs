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
pub struct MetricsCanister {
    /// Start at timestamp in nanoseconds, if 0 it will dump daily metrics
    /// since May 18, 2024
    pub start_at_timestamp: u64,

    /// Vector of subnets to query, if empty will dump metrics for
    /// all subnets
    pub subnet_ids: Vec<PrincipalId>,
}

impl ExecutableCommand for MetricsCanister {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let lock = Mutex::new(());
        let mut metrics_by_subnet = BTreeMap::new();

        let canister_agent = ctx.create_ic_agent_canister_client(Some(lock))?;
        info!("Started action...");



        println!("{}", serde_json::to_string_pretty(&metrics_by_subnet)?);

        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
