use std::{collections::BTreeMap, time::Duration};

use crate::calculation::should_proceed::should_proceed;
use chrono::{Local, NaiveDate};
use ic_management_backend::registry::RegistryState;
use ic_management_types::Subnet;
use prometheus_http_query::Client;
use serde::Deserialize;
use slog::{info, Logger};

use self::{
    release_actions::{create_current_release_feature_spec, find_latest_release},
    stage_checks::{check_stages, SubnetAction},
};

mod release_actions;
mod should_proceed;
mod stage_checks;

#[derive(Deserialize, Clone, Default)]
pub struct Index {
    pub rollout: Rollout,
    pub releases: Vec<Release>,
}

#[derive(Deserialize, Clone, Default)]
pub struct Rollout {
    #[serde(default)]
    pub pause: bool,
    pub skip_days: Vec<NaiveDate>,
    pub stages: Vec<Stage>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]

pub struct Stage {
    pub subnets: Vec<String>,
    #[serde(with = "humantime_serde")]
    pub bake_time: Duration,
    pub wait_for_next_week: bool,
    update_unassigned_nodes: bool,
}

#[derive(Deserialize, Clone, Default)]
pub struct Release {
    pub rc_name: String,
    pub versions: Vec<Version>,
}

#[derive(Deserialize, Clone, Default)]
pub struct Version {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub release_notes_read: bool,
    #[serde(default)]
    pub subnets: Vec<String>,
}

pub async fn calculate_progress<'a>(
    logger: &'a Logger,
    index: Index,
    prometheus_client: &'a Client,
    registry_state: RegistryState,
) -> anyhow::Result<Vec<SubnetAction>> {
    if !should_proceed(&index, Local::now().to_utc().date_naive()) {
        info!(logger, "Rollout controller paused or should skip this day.");
        return Ok(vec![]);
    }

    let latest_release = find_latest_release(&index)?;
    let elected_versions = registry_state.get_blessed_replica_versions().await?;

    let (current_version, current_feature_spec) =
        create_current_release_feature_spec(&latest_release, elected_versions)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut last_bake_status: BTreeMap<String, f64> = BTreeMap::new();
    let result = prometheus_client
        .query(
            r#"
                time() - max(last_over_time(
                    (timestamp(
                        sum by(ic_active_version,ic_subnet) (ic_replica_info)
                    ))[21d:1m]
                ) unless (sum by (ic_active_version, ic_subnet) (ic_replica_info))) by (ic_subnet)
                "#,
        )
        .get()
        .await?;

    let last = match result.data().clone().into_vector().into_iter().last() {
        Some(data) => data,
        None => return Err(anyhow::anyhow!("There should be data regarding ic_replica_info")),
    };

    for vector in last.iter() {
        let subnet = vector.metric().get("ic_subnet").expect("To have ic_subnet key");
        let last_update = vector.sample().value();
        last_bake_status.insert(subnet.to_string(), last_update);
    }

    let subnet_update_proposals = registry_state.open_subnet_upgrade_proposals().await?;
    let unassigned_nodes_version = registry_state.get_unassigned_nodes_replica_version().await?;

    let actions = check_stages(
        &current_version,
        &current_feature_spec,
        &last_bake_status,
        &subnet_update_proposals,
        &index.rollout.stages,
        Some(&logger),
        &unassigned_nodes_version,
        &registry_state.subnets().into_values().collect::<Vec<Subnet>>(),
    )?;

    Ok(actions)
}
