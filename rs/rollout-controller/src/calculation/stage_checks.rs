use std::{collections::BTreeMap, time::Duration};

use humantime::format_duration;
use ic_management_backend::proposal::SubnetUpdateProposal;
use ic_management_types::Subnet;
use slog::{debug, info, Logger};

use super::Stage;

pub fn check_stages(
    current_version: &String,
    current_release_feature_spec: BTreeMap<String, Vec<String>>,
    last_bake_status: BTreeMap<String, f64>,
    subnet_update_proposals: Vec<SubnetUpdateProposal>,
    stages: &Vec<Stage>,
    logger: Option<&Logger>,
    unassigned_version: &String,
    subnets: &Vec<Subnet>,
) -> anyhow::Result<Vec<String>> {
    for (i, stage) in stages.iter().enumerate() {
        if let Some(logger) = logger {
            info!(logger, "### Checking stage {}", i);
        }
        let mut actions = vec![];
        let mut stage_baked = true;

        if stage.update_unassigned_nodes {
            if let Some(logger) = logger {
                debug!(logger, "Unassigned nodes stage");
            }

            let unassigned_nodes_version = unassigned_version;

            if unassigned_nodes_version.eq(current_version) {
                if let Some(logger) = logger {
                    info!(logger, "Unassigned version already at version '{}'", current_version);
                }
                continue;
            }
            // Should update unassigned nodes
            actions.push(format!("Update unassigned nodes to version: {}", current_version));
            return Ok(actions);
        }

        if let Some(logger) = logger {
            debug!(logger, "Regular nodes stage");
        }
        for subnet_short in &stage.subnets {
            let desired_version = match current_release_feature_spec
                .iter()
                .find(|(_, subnets)| subnets.contains(subnet_short))
            {
                Some((name, _)) => name,
                None => current_version,
            };

            if let Some(logger) = logger {
                debug!(
                    logger,
                    "Checking if subnet {} is on desired version '{}'", subnet_short, desired_version
                );
            }

            let subnet = match subnets
                .into_iter()
                .find(|key| key.principal.to_string().starts_with(subnet_short))
            {
                Some(subnet) => subnet,
                None => {
                    return Err(anyhow::anyhow!(
                        "Couldn't find subnet that starts with '{}'",
                        subnet_short
                    ))
                }
            };

            if subnet.replica_version.eq(desired_version) {
                if let Some(logger) = logger {
                    info!(logger, "Subnet {} is at desired version", subnet_short);
                }
                // Check bake time
                let bake = last_bake_status
                    .get(&subnet.principal.to_string())
                    .expect("Should have key");
                if bake.gt(&stage.bake_time.as_secs_f64()) {
                    if let Some(logger) = logger {
                        info!(logger, "Subnet {} is baked", subnet_short);
                    }
                    continue;
                } else {
                    let remaining = Duration::from_secs_f64(stage.bake_time.as_secs_f64() - bake);
                    let formatted = format_duration(remaining);
                    if let Some(logger) = logger {
                        info!(
                            logger,
                            "Waiting for subnet {} to bake, pending {}", subnet_short, formatted
                        );
                    }
                    stage_baked |= false;
                }
            }
            if let Some(logger) = logger {
                info!(logger, "Subnet {} is not at desired version", subnet_short);
            }
            // Check if there is an open proposal => if yes, wait; if false, place and exit

            if let Some(proposal) = subnet_update_proposals.iter().find(|proposal| {
                proposal.payload.subnet_id == subnet.principal
                    && proposal.payload.replica_version_id.eq(desired_version)
                    && !proposal.info.executed
            }) {
                if let Some(logger) = logger {
                    info!(
                        logger,
                        "For subnet '{}' found open proposal 'https://dashboard.internetcomputer.org/proposal/{}'",
                        subnet_short,
                        proposal.info.id
                    );
                }
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

        if let Some(logger) = logger {
            info!(logger, "### Stage {} completed", i)
        }
    }
    if let Some(logger) = logger {
        info!(logger, "The current rollout '{}' is completed.", current_version);
    }
    Ok(vec![])
}
