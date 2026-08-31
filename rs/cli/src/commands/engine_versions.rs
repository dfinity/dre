use crate::commands::registry::engine_upgrade_priority;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use clap::{Args, error::ErrorKind};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use serde::Serialize;

/// Lists engine subnets whose upgrade priority falls within a given range.
///
/// Every engine (CloudEngine subnet that follows the standard upgrade train,
/// i.e. has a blank `replica_version_id`) is assigned a deterministic
/// pseudo-random priority in `[0.0, 1.0]` derived from its subnet id and the
/// standard engine's `new_replica_version_id`. An engine takes on the new
/// version once the standard `deployment_progress` reaches (or exceeds) its
/// priority.
///
/// This command reports the engines whose priority is inside the requested
/// range `(from, to]` (from exclusive, to inclusive).
#[derive(Args, Debug)]
#[clap(about = "List engine subnets whose upgrade priority falls within a range", visible_aliases = ["engine-version", "engines"])]
pub struct EngineVersions {
    /// Lower bound of the priority range (exclusive).
    #[clap(long, default_value_t = 0.0)]
    from: f64,

    /// Upper bound of the priority range (inclusive).
    #[clap(long, default_value_t = 1.0)]
    to: f64,

    /// Override the new replica version id used to compute priorities. Defaults
    /// to the `new_replica_version_id` from the registry's
    /// StandardEngineReplicaVersionRecord.
    #[clap(long)]
    new_replica_version_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct EngineInRange {
    subnet_id: PrincipalId,
    priority: f64,
}

#[derive(Debug, Serialize)]
struct EngineVersionsOutput {
    from: f64,
    to: f64,
    new_replica_version_id: String,
    engines: Vec<EngineInRange>,
}

impl ExecutableCommand for EngineVersions {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        if !(0.0..=1.0).contains(&self.from) {
            cmd.error(ErrorKind::InvalidValue, format!("--from must be within [0.0, 1.0], got {}", self.from))
                .exit()
        }
        if !(0.0..=1.0).contains(&self.to) {
            cmd.error(ErrorKind::InvalidValue, format!("--to must be within [0.0, 1.0], got {}", self.to))
                .exit()
        }
        if self.from > self.to {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("--from ({}) must not be greater than --to ({})", self.from, self.to),
            )
            .exit()
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;

        // Determine the new replica version id to compute priorities against.
        let new_replica_version_id = match &self.new_replica_version_id {
            Some(v) => v.clone(),
            None => {
                registry
                    .get_standard_engine_replica_version()?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No StandardEngineReplicaVersionRecord exists in the registry. Provide --new-replica-version-id to compute priorities manually."
                        )
                    })?
                    .new_replica_version_id
            }
        };

        let subnets = registry.subnets().await?;

        // Only CloudEngine subnets that follow the standard train (blank
        // replica_version) participate in the standard rollout.
        let engines = subnets
            .values()
            .filter(|subnet| subnet.subnet_type == SubnetType::CloudEngine && subnet.replica_version.is_empty())
            .map(|subnet| EngineInRange {
                subnet_id: subnet.principal,
                priority: engine_upgrade_priority(&subnet.principal, &new_replica_version_id),
            })
            // Range is (from, to]: `from` exclusive, `to` inclusive. This keeps
            // consecutive ranges (e.g. (0.1, 0.5] then (0.5, 0.9]) from double
            // counting the boundary engine. As a special case, `from == 0.0`
            // (the default lower bound) is treated as inclusive so an engine with
            // priority exactly 0.0 is still captured by the full default range.
            .filter(|engine| {
                let above_from = if self.from == 0.0 {
                    engine.priority >= 0.0
                } else {
                    engine.priority > self.from
                };
                above_from && engine.priority <= self.to
            })
            .sorted_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap_or(std::cmp::Ordering::Equal))
            .collect_vec();

        let output = EngineVersionsOutput {
            from: self.from,
            to: self.to,
            new_replica_version_id,
            engines,
        };

        println!("{}", serde_json::to_string_pretty(&output)?);

        Ok(())
    }
}
