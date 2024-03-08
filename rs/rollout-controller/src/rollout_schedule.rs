use std::{collections::BTreeMap, time::Duration};

use anyhow::Ok;
use chrono::{Local, NaiveDate};
use humantime::format_duration;
use ic_management_backend::registry::RegistryState;
use prometheus_http_query::Client;
use serde::Deserialize;
use slog::{debug, info, Logger};

pub async fn calculate_progress(
    logger: &Logger,
    index: Index,
    prometheus_client: &Client,
    registry_state: RegistryState,
) -> anyhow::Result<Vec<String>> {
    // Check if the plan is paused
    if index.rollout.pause {
        info!(logger, "Release is paused, no progress to be made.");
        return Ok(vec![]);
    }

    // Check if this day should be skipped
    let today = Local::now().to_utc().date_naive();
    if index.rollout.skip_days.iter().any(|f| f.eq(&today)) {
        info!(logger, "'{}' should be skipped as per rollout skip days", today);
        return Ok(vec![]);
    }

    debug!(logger, "Fetching elected versions");
    let elected_versions = registry_state.get_blessed_replica_versions().await?;
    let current_release = match index.releases.first() {
        Some(release) => release,
        None => return Err(anyhow::anyhow!("Expected to have atleast one release")),
    };

    debug!(logger, "Checking if versions are elected");
    let mut current_release_feature_spec: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_version = String::from("");
    for version in &current_release.versions {
        if !elected_versions.contains(&version.version) {
            return Err(anyhow::anyhow!(
                "Version '{}' is not blessed and is required for release '{}' with build name: {}",
                version.version,
                current_release.rc_name,
                version.name
            ));
        }

        if version.name.eq(&current_release.rc_name) {
            current_version = version.version.to_string();
            continue;
        }

        if !version.name.eq(&current_release.rc_name) && version.subnets.is_empty() {
            return Err(anyhow::anyhow!(
                "Feature build '{}' doesn't have subnets specified",
                version.name
            ));
        }

        current_release_feature_spec.insert(version.version.to_string(), version.subnets.clone());
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

    let subnet_update_proposals = registry_state.open_subnet_upgrade_proposals().await?;

    for (i, stage) in index.rollout.stages.iter().enumerate() {
        info!(logger, "### Checking stage {}", i);
        let mut actions = vec![];
        let mut stage_baked = true;

        if stage.update_unassigned_nodes {
            debug!(logger, "Unassigned nodes stage");

            let unassigned_nodes_version = registry_state.get_unassigned_nodes_replica_version().await?;

            if unassigned_nodes_version.eq(&current_version) {
                info!(logger, "Unassigned version already at version '{}'", current_version);
                continue;
            }
            // Should update unassigned nodes
            actions.push(format!("Update unassigned nodes to version: {}", current_version));
            return Ok(actions);
        }

        debug!(logger, "Regular nodes stage");
        for subnet_short in &stage.subnets {
            let desired_version = match current_release_feature_spec
                .iter()
                .find(|(_, subnets)| subnets.contains(subnet_short))
            {
                Some((name, _)) => name,
                None => &current_version,
            };
            debug!(
                logger,
                "Checking if subnet {} is on desired version '{}'", subnet_short, desired_version
            );

            let subnet = match registry_state
                .subnets()
                .into_iter()
                .find(|(key, _)| key.to_string().starts_with(subnet_short))
            {
                Some((_, subnet)) => subnet,
                None => {
                    return Err(anyhow::anyhow!(
                        "Couldn't find subnet that starts with '{}'",
                        subnet_short
                    ))
                }
            };

            if subnet.replica_version.eq(desired_version) {
                info!(logger, "Subnet {} is at desired version", subnet_short);
                // Check bake time
                let bake = last_bake_status
                    .get(&subnet.principal.to_string())
                    .expect("Should have key");
                if bake.gt(&stage.bake_time.as_secs_f64()) {
                    info!(logger, "Subnet {} is baked", subnet_short);
                    continue;
                } else {
                    let remaining = Duration::from_secs_f64(stage.bake_time.as_secs_f64() - bake);
                    let formatted = format_duration(remaining);
                    info!(
                        logger,
                        "Waiting for subnet {} to bake, pending {}", subnet_short, formatted
                    );
                    stage_baked |= false;
                }
            }
            info!(logger, "Subnet {} is not at desired version", subnet_short);
            // Check if there is an open proposal => if yes, wait; if false, place and exit

            if let Some(proposal) = subnet_update_proposals.iter().find(|proposal| {
                proposal.payload.subnet_id == subnet.principal
                    && proposal.payload.replica_version_id.eq(desired_version)
                    && !proposal.info.executed
            }) {
                info!(
                    logger,
                    "For subnet '{}' found open proposal 'https://dashboard.internetcomputer.org/proposal/{}'",
                    subnet_short,
                    proposal.info.id
                );
                continue;
            }

            actions.push(format!(
                "Should update subnet '{}' to version '{}'",
                subnet_short, desired_version
            ))
        }

        if !stage_baked {
            return Ok(vec![]);
        }

        if !actions.is_empty() {
            return Ok(actions);
        }

        info!(logger, "### Stage {} completed", i)
    }

    // Iterate over stages and compare desired state from file and the state it is live
    // Take into consideration:
    //      1. If the subnet is on the desired version, proceed
    //      2. If the subnet isn't on the desired version:
    //          2.1. Check if there is an open proposal for upgrading, if there isn't place one
    //          2.2. If the proposal is executed check when it was upgraded and query prometheus
    //               to see if there were any alerts and if the bake time has passed. If the bake
    //               time didn't pass don't do anything. If there were alerts don't do anything.

    Ok(vec![])
}

#[derive(Deserialize)]
pub struct Index {
    pub rollout: Rollout,
    pub releases: Vec<Release>,
}

#[derive(Deserialize)]
pub struct Rollout {
    #[serde(default)]
    pub pause: bool,
    pub skip_days: Vec<NaiveDate>,
    pub stages: Vec<Stage>,
}

#[derive(Deserialize, Default)]
#[serde(default)]

pub struct Stage {
    pub subnets: Vec<String>,
    #[serde(with = "humantime_serde")]
    pub bake_time: Duration,
    pub wait_for_next_week: bool,
    update_unassigned_nodes: bool,
}

#[derive(Deserialize)]
pub struct Release {
    pub rc_name: String,
    pub versions: Vec<Version>,
}

#[derive(Deserialize)]
pub struct Version {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub release_notes_read: bool,
    #[serde(default)]
    pub subnets: Vec<String>,
}
