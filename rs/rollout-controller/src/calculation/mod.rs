use std::{collections::BTreeMap, time::Duration};

use crate::calculation::should_proceed::should_proceed;
use chrono::{Local, NaiveDate, TimeDelta};
use ic_management_backend::registry::RegistryState;
use ic_management_types::Subnet;
use itertools::Itertools;
use prometheus_http_query::Client;
use serde::Deserialize;
use slog::{info, Logger};

use self::stage_checks::{check_stages, desired_rollout_release_version};
use crate::actions::SubnetAction;

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

#[derive(Deserialize, Clone, Default, Eq, PartialEq, Hash, Debug)]
pub struct Release {
    pub rc_name: String,
    pub versions: Vec<Version>,
}

#[derive(Deserialize, Clone, Default, Eq, PartialEq, Hash, Debug)]
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

    let subnets = registry_state.subnets().into_values().collect::<Vec<Subnet>>();
    let desired_versions = desired_rollout_release_version(&subnets, &index.releases);
    let concatenated_versions = desired_versions
        .release
        .versions
        .iter()
        .map(|v| v.version.clone())
        .join("|");

    let result = prometheus_client.query(format!(r#"
    time() - first_over_time((timestamp(group(ic_replica_info{{ic_active_version=~"{concatenated_versions}"}})))[14d:1d])
    "#)).get().await?;

    let since_start = match result.data().clone().into_vector().into_iter().last() {
        Some(data) => match data.iter().last() {
            Some(data) => data.sample().value(),
            None => {
                return Err(anyhow::anyhow!(
                    "There should be data regarding start of releases in response vector"
                ))
            }
        },
        None => return Err(anyhow::anyhow!("There should be data regarding start of releases")),
    };

    let subnet_update_proposals = registry_state.open_subnet_upgrade_proposals().await?;
    let unassigned_nodes_version = registry_state.get_unassigned_nodes_replica_version().await?;
    let unassigned_nodes_proposals = registry_state.open_upgrade_unassigned_nodes_proposals().await?;

    let actions = check_stages(
        &last_bake_status,
        &subnet_update_proposals,
        &unassigned_nodes_proposals,
        index,
        Some(&logger),
        &unassigned_nodes_version,
        &subnets,
        Local::now().date_naive(),
        Local::now()
            .checked_sub_signed(
                TimeDelta::try_seconds(since_start as i64).expect("Should be able to convert to seconds"),
            )
            .expect("Should be able to sub from now")
            .date_naive(),
        desired_versions,
    )?;

    Ok(actions)
}
