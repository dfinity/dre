use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use anyhow::Ok;
use chrono::{Days, Local, NaiveDate};
use humantime::format_duration;
use ic_management_backend::registry::RegistryState;
use ic_management_types::Network;
use prometheus_http_query::Client;
use serde::Deserialize;
use slog::{debug, info, Logger};
use tokio::{fs::File, io::AsyncReadExt, select};
use tokio_util::sync::CancellationToken;

pub async fn calculate_progress(
    logger: &Logger,
    release_index: &PathBuf,
    network: &Network,
    token: CancellationToken,
    prometheus_client: &Client,
) -> anyhow::Result<()> {
    // Deserialize the index
    debug!(logger, "Deserializing index");
    let mut index = String::from("");
    let mut rel_index = File::open(release_index)
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't open release index: {:?}", e))?;
    rel_index
        .read_to_string(&mut index)
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't read release index: {:?}", e))?;
    let index: Index =
        serde_yaml::from_str(&index).map_err(|e| anyhow::anyhow!("Couldn't parse release indes: {:?}", e))?;

    // Check if the plan is paused
    if index.rollout.pause {
        info!(logger, "Release is paused, no progress to be made.");
        return Ok(());
    }

    // Check if this day should be skipped
    let today = Local::now().to_utc().date_naive();
    if index.rollout.skip_days.iter().any(|f| f.eq(&today)) {
        info!(logger, "'{}' should be skipped as per rollout skip days", today);
        return Ok(());
    }

    // Check if the desired rollout version is elected
    debug!(logger, "Creating registry");
    let mut registry_state = select! {
        res = RegistryState::new(network.clone(), true) => res,
        _ = token.cancelled() => {
            info!(logger, "Received shutdown while creating registry");
            return Ok(())
        }
    };
    debug!(logger, "Updating registry with data");
    let node_provider_data = vec![];
    select! {
        res = registry_state.update_node_details(&node_provider_data) => res?,
        _ = token.cancelled() => {
            info!(logger, "Received shutdown while creating registry");
            return Ok(())
        }
    }
    debug!(
        logger,
        "Created registry with latest version: '{}'",
        registry_state.version()
    );
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
    for (i, stage) in index.rollout.stages.iter().enumerate() {
        info!(logger, "### Checking stage {}", i);

        if stage.update_unassigned_nodes {
            debug!(logger, "Unassigned nodes stage");

            let unassigned_nodes_version = registry_state.get_unassigned_nodes_replica_version().await?;

            if unassigned_nodes_version.eq(&current_version) {
                info!(logger, "Unassigned version already at version '{}'", current_version);
                continue;
            }
            // Update unassigned nodes
        }

        debug!(logger, "Regular nodes stage");
        for subnet_short in &stage.subnets {
            let desired_version = match current_release_feature_spec
                .iter()
                .find(|(_, subnets)| subnets.contains(&subnet_short))
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
                let now = Local::now().to_utc();
                let principal = subnet.principal.to_string();
                let result = prometheus_client
                    .query_range(
                        format!(
                            r#"
last_over_time(
    (timestamp(
        sum by(ic_active_version,ic_subnet) (ic_replica_info{{ic_subnet="{0}", ic_active_version="{1}"}})
    ))[7d:1m]
)
"#,
                            principal, desired_version
                        ),
                        now.checked_sub_days(Days::new(7)).unwrap().timestamp(),
                        now.timestamp(),
                        10.0,
                    )
                    .get()
                    .await?;

                if let Some(range) = result.data().as_matrix() {
                    let first = match range.first() {
                        Some(val) => match val.samples().first() {
                            Some(val) => val.timestamp(),
                            None => {
                                return Err(anyhow::anyhow!(
                                    "Expected samples to have first data for bake time for subnet '{}'",
                                    subnet_short,
                                ))
                            }
                        },
                        None => {
                            return Err(anyhow::anyhow!(
                                "Expected range vector to have first data for bake time for subnet '{}'",
                                subnet_short
                            ))
                        }
                    };

                    let last = match range.last() {
                        Some(val) => match val.samples().last() {
                            Some(val) => val.timestamp(),
                            None => {
                                return Err(anyhow::anyhow!(
                                    "Expected samples to have last data for bake time for subnet '{}'",
                                    subnet_short
                                ))
                            }
                        },
                        None => {
                            return Err(anyhow::anyhow!(
                                "Expected range vector to have last data for bake time for subnet '{}'",
                                subnet_short
                            ))
                        }
                    };

                    let diff = last - first;

                    debug!(
                        logger,
                        "For subnet '{}' comparing bake time - {} {} - diff",
                        subnet_short,
                        stage.bake_time.as_secs_f64(),
                        diff
                    );

                    if diff > stage.bake_time.as_secs_f64() {
                        info!(logger, "Subnet {} is baked", subnet_short);
                        continue;
                    } else {
                        let remaining = Duration::from_secs_f64(stage.bake_time.as_secs_f64() - diff);
                        let formatted = format_duration(remaining);
                        info!(
                            logger,
                            "Waiting for subnet {} to bake, pending {}", subnet_short, formatted
                        );
                        return Ok(());
                    }
                } else {
                    return Err(anyhow::anyhow!("Expected result data to be a matrix"));
                }
            }
            info!(logger, "Subnet {} is not at desired version", subnet_short)
            // Check if there is an open proposal => if yes, wait; if false, place and exit
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

    Ok(())
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
