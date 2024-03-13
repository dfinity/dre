use std::{collections::BTreeMap, time::Duration};

use chrono::{Datelike, Days, NaiveDate, Weekday};
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
    WaitForNextWeek {
        subnet_short: String,
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
    start_of_release: NaiveDate,
    now: NaiveDate,
) -> anyhow::Result<Vec<SubnetAction>> {
    for (i, stage) in stages.iter().enumerate() {
        if let Some(logger) = logger {
            info!(logger, "Checking stage {}", i)
        }

        if stage.wait_for_next_week && !week_passed(start_of_release, now) {
            let actions = stage
                .subnets
                .iter()
                .map(|subnet| SubnetAction::WaitForNextWeek {
                    subnet_short: subnet.to_string(),
                })
                .collect();
            return Ok(actions);
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

fn week_passed(release_start: NaiveDate, now: NaiveDate) -> bool {
    let counter = release_start.clone();
    counter
        .checked_add_days(Days::new(1))
        .expect("Should be able to add a day");
    while counter <= now {
        if counter.weekday() == Weekday::Mon {
            return true;
        }
        counter
            .checked_add_days(Days::new(1))
            .expect("Should be able to add a day");
    }
    false
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
    subnet_short: &'a str,
    current_release_feature_spec: &'a BTreeMap<String, Vec<String>>,
    current_version: &'a str,
) -> &'a str {
    return match current_release_feature_spec
        .iter()
        .find(|(_, subnets)| subnets.contains(&subnet_short.to_string()))
    {
        Some((name, _)) => name,
        None => current_version,
    };
}

fn find_subnet_by_short_id<'a>(subnets: &'a [Subnet], subnet_short: &'a str) -> anyhow::Result<&'a Subnet> {
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

    return match bake.ge(&stage_bake_time) {
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
    use rstest::rstest;

    pub(super) fn craft_subnet_from_id<'a>(subnet_id: &'a str) -> Subnet {
        Subnet {
            principal: PrincipalId(Principal::from_str(subnet_id).expect("Can create principal")),
            ..Default::default()
        }
    }

    pub(super) fn craft_subnet_from_similar_id<'a>(subnet_id: &'a str) -> Subnet {
        let original_principal = Principal::from_str(subnet_id).expect("Can create principal");
        let mut new_principal = original_principal.as_slice().to_vec();
        let len = new_principal.len();
        if let Some(byte) = new_principal.get_mut(len - 1) {
            *byte += 1;
        }
        Subnet {
            principal: PrincipalId(Principal::try_from_slice(&new_principal[..]).expect("Can create principal")),
            ..Default::default()
        }
    }

    pub(super) fn craft_proposals<'a>(
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

    pub(super) fn craft_open_proposals<'a>(subnet_ids: &'a [&'a str], version: &'a str) -> Vec<SubnetUpdateProposal> {
        craft_proposals(
            &subnet_ids.iter().map(|id| (*id, false)).collect::<Vec<(&str, bool)>>(),
            version,
        )
        .collect()
    }

    pub(super) fn craft_executed_proposals<'a>(
        subnet_ids: &'a [&'a str],
        version: &'a str,
    ) -> Vec<SubnetUpdateProposal> {
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

        let subnet = craft_subnet_from_id("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae");
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, "version");

        assert!(proposal.is_some())
    }

    #[rstest]
    #[case(
        "version",
        "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
        "version"
    )]
    #[case(
        "other-version",
        "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
        "version"
    )]
    #[case(
        "version",
        "5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae",
        "version"
    )]
    fn should_not_find_open_proposal(
        #[case] proposal_version: &str,
        #[case] subnet_id: &str,
        #[case] current_version: &str,
    ) {
        let proposals = craft_executed_proposals(
            &vec![
                "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
                "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            ],
            proposal_version,
        );
        let subnet = craft_subnet_from_id(subnet_id);
        let proposal = get_open_proposal_for_subnet(&proposals, &subnet, current_version);

        assert!(proposal.is_none())
    }
}

#[cfg(test)]
mod get_remaining_bake_time_for_subnet_tests {
    use super::*;
    use rstest::rstest;

    fn craft_bake_status_from_tuples(tuples: &[(&str, f64)]) -> BTreeMap<String, f64> {
        tuples
            .iter()
            .map(|(id, bake_time)| (id.to_string(), *bake_time))
            .collect::<BTreeMap<String, f64>>()
    }

    #[test]
    fn should_return_error_subnet_not_found() {
        let subnet = get_open_proposal_for_subnet_tests::craft_subnet_from_id(
            "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
        );

        let bake_status = craft_bake_status_from_tuples(&[("random-subnet", 1.0)]);

        let maybe_remaining_bake_time = get_remaining_bake_time_for_subnet(&bake_status, &subnet, 100.0);

        assert!(maybe_remaining_bake_time.is_err())
    }

    #[rstest]
    #[case(100.0, 100.0, 0.0)]
    #[case(150.0, 100.0, 0.0)]
    #[case(100.0, 150.0, 50.0)]
    // Should these be allowed? Technically we will never get
    // something like this from prometheus and there should
    // be validation for incoming configuration, but it is a
    // possibility in our code. Maybe we could add validation
    // checks that disallow of negative baking time?
    #[case(-100.0, 150.0, 250.0)]
    #[case(-100.0, -150.0, 0.0)]
    #[case(-100.0, -50.0, 50.0)]
    fn should_return_subnet_baking_time(
        #[case] subnet_bake_status: f64,
        #[case] stage_bake: f64,
        #[case] remaining: f64,
    ) {
        let subnet = get_open_proposal_for_subnet_tests::craft_subnet_from_id(
            "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
        );

        let bake_status = craft_bake_status_from_tuples(&[(
            "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            subnet_bake_status,
        )]);

        let maybe_remaining_bake_time = get_remaining_bake_time_for_subnet(&bake_status, &subnet, stage_bake);

        assert!(maybe_remaining_bake_time.is_ok());
        let remaining_bake_time = maybe_remaining_bake_time.unwrap();
        assert_eq!(remaining_bake_time, remaining)
    }
}

#[cfg(test)]
mod find_subnet_by_short_id_tests {
    use self::get_open_proposal_for_subnet_tests::{craft_subnet_from_id, craft_subnet_from_similar_id};

    use super::*;

    #[test]
    fn should_find_subnet() {
        let subnet_1 = craft_subnet_from_id("5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae");
        let subnet_2 = craft_subnet_from_id("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae");
        let subnet_3 = craft_subnet_from_similar_id("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae");
        let subnets = &[subnet_1, subnet_2, subnet_3];

        let maybe_subnet = find_subnet_by_short_id(subnets, "snjp4");

        assert!(maybe_subnet.is_ok());
        let subnet = maybe_subnet.unwrap();
        let subnet_principal = subnet.principal.to_string();
        assert!(subnet_principal.starts_with("snjp4"));
        assert!(subnet_principal.eq("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae"));
    }

    #[test]
    fn should_find_not_subnet() {
        let subnet_1 = craft_subnet_from_id("5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae");
        let subnet_2 = craft_subnet_from_id("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae");
        let subnets = &[subnet_1, subnet_2];

        let maybe_subnet = find_subnet_by_short_id(subnets, "random-subnet");

        assert!(maybe_subnet.is_err());
    }
}

#[cfg(test)]
mod get_desired_version_for_subnet_test {
    use super::*;

    pub(super) fn craft_feature_spec(tuples: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        tuples
            .iter()
            .map(|(commit, subnets)| (commit.to_string(), subnets.iter().map(|id| id.to_string()).collect()))
            .collect::<BTreeMap<String, Vec<String>>>()
    }

    #[test]
    fn should_return_current_version() {
        let current_release_feature_spec =
            craft_feature_spec(&[("feature-a-commit", &["subnet-1", "subnet-2", "subnet-3"])]);

        let version = get_desired_version_for_subnet("subnet", &current_release_feature_spec, "current_version");
        assert_eq!(version, "current_version")
    }

    #[test]
    fn should_return_current_version_empty_feature_spec() {
        let current_release_feature_spec = craft_feature_spec(&vec![]);

        let version = get_desired_version_for_subnet("subnet", &current_release_feature_spec, "current_version");
        assert_eq!(version, "current_version")
    }

    #[test]
    fn should_return_feature_version() {
        let current_release_feature_spec =
            craft_feature_spec(&[("feature-a-commit", &["subnet-1", "subnet-2", "subnet-3"])]);

        let version = get_desired_version_for_subnet("subnet-1", &current_release_feature_spec, "current_version");
        assert_eq!(version, "feature-a-commit")
    }
}

// E2E tests for decision making process for happy path without feature builds
#[cfg(test)]
mod check_stages_tests_no_feature_builds {
    use crate::calculation::{Index, Release, Rollout, Version};

    use super::*;

    /// Part one => No feature builds
    /// `current_version` - can be defined
    /// `current_release_feature_spec` - empty because we don't have feature builds for this part
    /// `last_bake_status` - can be defined
    /// `subnet_update_proposals` - can be defined
    /// `stages` - must be defined
    /// `logger` - can be defined, but won't be because these are only tests
    /// `unassigned_version` - should be defined
    /// `subnets` - should be defined
    ///
    /// For all use cases we will use the following setup
    /// rollout:
    ///     pause: false // Tested in `should_proceed.rs` module
    ///     skip_days: [] // Tested in `should_proceed.rs` module
    ///     stages:
    ///         - subnets: [io67a]
    ///           bake_time: 8h
    ///         - subnets: [shefu, uzr34]
    ///           bake_time: 4h
    ///         - update_unassigned_nodes: true
    ///         - subnets: [pjljw]
    ///           wait_for_next_week: true
    ///           bake_time: 4h
    /// releases:
    ///     - rc_name: rc--2024-02-21_23-01
    ///       versions:
    ///         - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
    ///           name: rc--2024-02-21_23-01
    ///           release_notes_ready: <not-important>
    ///           subnets: [] // empty because its a regular build
    ///     - rc_name: rc--2024-02-14_23-01
    ///       versions:
    ///         - version: 85bd56a70e55b2cea75cae6405ae11243e5fdad8
    ///           name: rc--2024-02-14_23-01
    ///           release_notes_ready: <not-important>
    ///           subnets: [] // empty because its a regular build
    fn craft_index_state() -> Index {
        Index {
            rollout: Rollout {
                pause: false,
                skip_days: [],
                stages: [
                    Stage {
                        subnets: ["io67a".to_string()],
                        bake_time: humantime::parse_duration("8h").expect("Should be able to parse."),
                        ..Default::default()
                    },
                    Stage {
                        subnets: ["shefu".to_string(), "uzr34".to_string()],
                        bake_time: humantime::parse_duration("4h").expect("Should be able to parse."),
                        ..Default::default()
                    },
                    Stage {
                        update_unassigned_nodes: true,
                        ..Default::default()
                    },
                    Stage {
                        subnets: ["pjljw".to_string()],
                        bake_time: humantime::parse_duration("4h").expect("Should be able to parse."),
                        wait_for_next_week: true,
                        ..Default::default()
                    },
                ],
            },
            releases: [
                Release {
                    rc_name: "rc--2024-02-21_23-01".to_string(),
                    versions: [Version {
                        name: "rc--2024-02-21_23-01".to_string(),
                        version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                        ..Default::default()
                    }],
                },
                Release {
                    rc_name: "rc--2024-02-14_23-01".to_string(),
                    versions: [Version {
                        name: "rc--2024-02-14_23-01".to_string(),
                        version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                        ..Default::default()
                    }],
                },
            ],
        }
    }

    /// Use-Case 1: Beginning of a new rollout
    ///
    /// `current_version` - set to a commit that is being rolled out
    /// `last_bake_status` - empty, because no subnets have the version
    /// `subnet_update_proposals` - can be empty but doesn't have to be. For e.g. if its Monday it is possible to have an open proposal for NNS
    ///                             But it is for a different version (one from last week)
    /// `unassigned_version` - one from previous week
    /// `subnets` - can be seen in `craft_index_state`
    #[test]
    fn test_rollout_beginning() {}
}

// E2E tests for decision making process for happy path with feature builds
#[cfg(test)]
mod check_stages_tests_feature_builds {}
