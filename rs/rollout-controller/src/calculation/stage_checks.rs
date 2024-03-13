use std::{collections::BTreeMap, time::Duration};

use humantime::format_duration;
use ic_management_backend::proposal::SubnetUpdateProposal;
use ic_management_types::Subnet;
use slog::{debug, info, Logger};

use super::Stage;

#[derive(Debug)]
pub enum SubnetAction {
    Noop {
        subnet_short: String,
    },
    Baking {
        subnet_short: String,
        remaining: Duration,
    },
    PendingProposal {
        subnet_short: String,
        proposal_id: u64,
    },
    PlaceProposal {
        is_unassigned: bool,
        subnet_principal: String,
        version: String,
    },
}

pub fn check_stages<'a>(
    current_version: &'a String,
    current_release_feature_spec: &'a BTreeMap<String, Vec<String>>,
    last_bake_status: &'a BTreeMap<String, f64>,
    subnet_update_proposals: &'a [SubnetUpdateProposal],
    stages: &'a [Stage],
    logger: Option<&'a Logger>,
    unassigned_version: &'a String,
    subnets: &'a [Subnet],
) -> anyhow::Result<Vec<SubnetAction>> {
    for (i, stage) in stages.iter().enumerate() {
        if let Some(logger) = logger {
            info!(logger, "Checking stage {}", i)
        }
        let stage_actions = check_stage(
            current_version,
            current_release_feature_spec,
            last_bake_status,
            subnet_update_proposals,
            stage,
            logger,
            unassigned_version,
            subnets,
        )?;

        if !stage_actions.iter().all(|a| {
            if let SubnetAction::Noop { subnet_short: _ } = a {
                return true;
            }
            return false;
        }) {
            return Ok(stage_actions);
        }

        if let Some(logger) = logger {
            info!(logger, "Stage {} is completed", i)
        }
    }

    if let Some(logger) = logger {
        info!(logger, "The current rollout '{}' is completed.", current_version);
    }

    Ok(vec![])
}

fn check_stage<'a>(
    current_version: &'a String,
    current_release_feature_spec: &'a BTreeMap<String, Vec<String>>,
    last_bake_status: &'a BTreeMap<String, f64>,
    subnet_update_proposals: &'a [SubnetUpdateProposal],
    stage: &'a Stage,
    logger: Option<&'a Logger>,
    unassigned_version: &'a String,
    subnets: &'a [Subnet],
) -> anyhow::Result<Vec<SubnetAction>> {
    let mut stage_actions = vec![];
    if stage.update_unassigned_nodes {
        // Update unassigned nodes
        if let Some(logger) = logger {
            debug!(logger, "Unassigned nodes stage");
        }

        if !unassigned_version.eq(current_version) {
            stage_actions.push(SubnetAction::PlaceProposal {
                is_unassigned: true,
                subnet_principal: "".to_string(),
                version: current_version.clone(),
            });
            return Ok(stage_actions);
        }
    }

    for subnet_short in &stage.subnets {
        // Get desired version
        let desired_version =
            get_desired_version_for_subnet(subnet_short, current_release_feature_spec, current_version);

        // Find subnet to by the subnet_short
        let subnet = find_subnet_by_short_id(subnets, subnet_short)?;

        if let Some(logger) = logger {
            debug!(
                logger,
                "Checking if subnet {} is on desired version '{}'", subnet_short, desired_version
            );
        }

        // If subnet is on desired version, check bake time
        if subnet.replica_version.eq(desired_version) {
            let remaining =
                get_remaining_bake_time_for_subnet(last_bake_status, subnet, stage.bake_time.as_secs_f64())?;
            let remaining_duration = Duration::from_secs_f64(remaining);
            let formatted = format_duration(remaining_duration);

            if remaining != 0.0 {
                stage_actions.push(SubnetAction::Baking {
                    subnet_short: subnet_short.clone(),
                    remaining: remaining_duration,
                })
            }

            if let Some(logger) = logger {
                if remaining == 0.0 {
                    debug!(logger, "Subnet {} baked", subnet_short)
                } else {
                    debug!(
                        logger,
                        "Waiting for subnet {} to bake, remaining {}", subnet_short, formatted
                    )
                }
            }

            stage_actions.push(SubnetAction::Noop {
                subnet_short: subnet_short.clone(),
            })
        }

        // If subnet is not on desired version, check if there is an open proposal
        if let Some(proposal) = get_open_proposal_for_subnet(subnet_update_proposals, subnet, desired_version) {
            if let Some(logger) = logger {
                info!(
                    logger,
                    "For subnet '{}' found open proposal with id '{}'", subnet_short, proposal.info.id
                )
            }
            stage_actions.push(SubnetAction::PendingProposal {
                subnet_short: subnet_short.clone(),
                proposal_id: proposal.info.id,
            })
        }

        // If subnet is not on desired version and there is no open proposal submit it
        stage_actions.push(SubnetAction::PlaceProposal {
            is_unassigned: false,
            subnet_principal: subnet.principal.to_string(),
            version: current_version.clone(),
        })
    }

    Ok(stage_actions)
}

fn get_desired_version_for_subnet<'a>(
    subnet_short: &'a String,
    current_release_feature_spec: &'a BTreeMap<String, Vec<String>>,
    current_version: &'a String,
) -> &'a String {
    return match current_release_feature_spec
        .iter()
        .find(|(_, subnets)| subnets.contains(subnet_short))
    {
        Some((name, _)) => name,
        None => current_version,
    };
}

fn find_subnet_by_short_id<'a>(subnets: &'a [Subnet], subnet_short: &'a String) -> anyhow::Result<&'a Subnet> {
    return match subnets
        .iter()
        .find(|s| s.principal.to_string().starts_with(subnet_short))
    {
        Some(subnet) => Ok(subnet),
        None => Err(anyhow::anyhow!("No subnet with short id '{}'", subnet_short)),
    };
}

fn get_remaining_bake_time_for_subnet(
    last_bake_status: &BTreeMap<String, f64>,
    subnet: &Subnet,
    stage_bake_time: f64,
) -> anyhow::Result<f64> {
    let bake = match last_bake_status.get(&subnet.principal.to_string()) {
        Some(bake) => bake,
        None => {
            return Err(anyhow::anyhow!(
                "Subnet with principal '{}' not found",
                subnet.principal.to_string()
            ))
        }
    };

    return match bake.gt(&stage_bake_time) {
        true => Ok(0.0),
        false => {
            let remaining = Duration::from_secs_f64(stage_bake_time - bake);
            return Ok(remaining.as_secs_f64());
        }
    };
}

fn get_open_proposal_for_subnet<'a>(
    subnet_update_proposals: &'a [SubnetUpdateProposal],
    subnet: &'a Subnet,
    desired_version: &'a str,
) -> Option<&'a SubnetUpdateProposal> {
    subnet_update_proposals.iter().find(|p| {
        p.payload.subnet_id == subnet.principal && p.payload.replica_version_id.eq(desired_version) && !p.info.executed
    })
}

#[cfg(test)]
mod get_open_proposal_for_subnet_tests {
    use std::str::FromStr;

    use candid::Principal;
    use ic_base_types::PrincipalId;
    use ic_management_backend::proposal::ProposalInfoInternal;
    use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;

    use super::*;

    fn craft_proposals<'a>(
        subnet_with_execution_status: &'a [(&'a str, bool)],
        version: &'a str,
    ) -> impl Iterator<Item = SubnetUpdateProposal> + 'a {
        subnet_with_execution_status
            .iter()
            .enumerate()
            .map(|(i, (id, executed))| SubnetUpdateProposal {
                payload: UpdateSubnetReplicaVersionPayload {
                    subnet_id: PrincipalId(Principal::from_str(id).expect("Can create principal")),
                    replica_version_id: version.to_string(),
                },
                info: ProposalInfoInternal {
                    id: i as u64,
                    // These values are not important for the function
                    executed_timestamp_seconds: 1,
                    proposal_timestamp_seconds: 1,
                    executed: *executed,
                },
            })
    }

    fn craft_open_proposals<'a>(subnet_ids: &'a [&'a str], version: &'a str) -> Vec<SubnetUpdateProposal> {
        craft_proposals(
            &subnet_ids.iter().map(|id| (*id, false)).collect::<Vec<(&str, bool)>>(),
            version,
        )
        .collect()
    }

    fn craft_executed_proposals<'a>(subnet_ids: &'a [&'a str], version: &'a str) -> Vec<SubnetUpdateProposal> {
        craft_proposals(
            &subnet_ids.iter().map(|id| (*id, true)).collect::<Vec<(&str, bool)>>(),
            version,
        )
        .collect()
    }

    #[test]
    fn should_find_open_proposal_for_subnet() {
        let proposals = craft_open_proposals(
            &vec![
                "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
                "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            ],
            "version",
        );

        let subnet = Subnet {
            principal: PrincipalId(
                Principal::from_str("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae")
                    .expect("Can create principal"),
            ),
            ..Default::default()
        };
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, "version");

        assert!(proposal.is_some())
    }

    #[test]
    fn should_not_find_open_proposal_all_are_executed() {
        let proposals = craft_executed_proposals(
            &vec![
                "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
                "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            ],
            "version",
        );
        let subnet = Subnet {
            principal: PrincipalId(
                Principal::from_str("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae")
                    .expect("Can create principal"),
            ),
            ..Default::default()
        };
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, "version");

        assert!(proposal.is_none())
    }

    #[test]
    fn should_not_find_open_proposal_all_are_executed_for_different_version() {
        let proposals = craft_executed_proposals(
            &vec![
                "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
                "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            ],
            "other-version",
        );
        let subnet = Subnet {
            principal: PrincipalId(
                Principal::from_str("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae")
                    .expect("Can create principal"),
            ),
            ..Default::default()
        };
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, "version");

        assert!(proposal.is_none())
    }

    #[test]
    fn should_not_find_open_proposal_all_are_executed_for_different_subnets() {
        let proposals = craft_executed_proposals(
            &vec![
                "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
                "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            ],
            "version",
        );
        let subnet = Subnet {
            principal: PrincipalId(
                Principal::from_str("5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae")
                    .expect("Can create principal"),
            ),
            ..Default::default()
        };
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, "version");

        assert!(proposal.is_none())
    }
}

#[cfg(test)]
mod get_remaining_bake_time_for_subnet_tests {
    use super::*;
}
