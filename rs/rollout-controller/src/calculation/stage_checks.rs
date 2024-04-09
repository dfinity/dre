use std::{collections::BTreeMap, time::Duration};

use crate::actions::SubnetAction;
use chrono::{Datelike, Days, NaiveDate, Weekday};
use humantime::format_duration;
use ic_base_types::PrincipalId;
use ic_management_backend::proposal::{SubnetUpdateProposal, UpdateUnassignedNodesProposal};
use ic_management_types::Subnet;
use itertools::Itertools;
use slog::{debug, info, Logger};

use super::{Index, Stage};

/// For the set of inputs, generate a vector of `SubnetAction`'s for an arbitrary stage.
/// All produced actions are always related to the same stage of an index rollout.
/// To find out more take a look at the e2e tests
pub fn check_stages<'a>(
    last_bake_status: &'a BTreeMap<String, f64>,
    subnet_update_proposals: &'a [SubnetUpdateProposal],
    unassigned_node_update_proposals: &'a [UpdateUnassignedNodesProposal],
    index: Index,
    logger: Option<&'a Logger>,
    unassigned_version: &'a String,
    subnets: &'a [Subnet],
    now: NaiveDate,
    start_of_release: NaiveDate,
    desired_versions: DesiredReleaseVersion,
) -> anyhow::Result<Vec<SubnetAction>> {
    for (i, stage) in index.rollout.stages.iter().enumerate() {
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
            last_bake_status,
            subnet_update_proposals,
            unassigned_node_update_proposals,
            stage,
            logger,
            unassigned_version,
            subnets,
            desired_versions.clone(),
        )?;

        if !stage_actions.iter().all(|a| {
            if let SubnetAction::Noop { subnet_short: _ } = a {
                return true;
            }
            false
        }) {
            return Ok(stage_actions);
        }

        if let Some(logger) = logger {
            info!(logger, "Stage {} is completed", i)
        }
    }

    if let Some(logger) = logger {
        info!(
            logger,
            "The current rollout '{}' is completed.", desired_versions.release.rc_name
        );
    }

    Ok(vec![])
}

fn week_passed(release_start: NaiveDate, now: NaiveDate) -> bool {
    let mut counter = release_start;
    counter = counter
        .checked_add_days(Days::new(1))
        .expect("Should be able to add a day");
    while counter <= now {
        if counter.weekday() == Weekday::Mon {
            return true;
        }
        counter = counter
            .checked_add_days(Days::new(1))
            .expect("Should be able to add a day");
    }
    false
}

fn check_stage<'a>(
    last_bake_status: &'a BTreeMap<String, f64>,
    subnet_update_proposals: &'a [SubnetUpdateProposal],
    unassigned_node_update_proposals: &'a [UpdateUnassignedNodesProposal],
    stage: &'a Stage,
    logger: Option<&'a Logger>,
    unassigned_version: &'a String,
    subnets: &'a [Subnet],
    desired_versions: DesiredReleaseVersion,
) -> anyhow::Result<Vec<SubnetAction>> {
    let mut stage_actions = vec![];
    if stage.update_unassigned_nodes {
        // Update unassigned nodes
        if let Some(logger) = logger {
            debug!(logger, "Unassigned nodes stage");
        }

        if *unassigned_version != desired_versions.unassigned_nodes.version {
            match unassigned_node_update_proposals.iter().find(|proposal| {
                if !proposal.info.executed {
                    if let Some(version) = &proposal.payload.replica_version {
                        if *version == desired_versions.unassigned_nodes.version {
                            return true;
                        }
                    }
                }
                false
            }) {
                None => stage_actions.push(SubnetAction::PlaceProposal {
                    is_unassigned: true,
                    subnet_principal: PrincipalId::new_anonymous(),
                    version: desired_versions.unassigned_nodes.version,
                }),
                Some(proposal) => stage_actions.push(SubnetAction::PendingProposal {
                    subnet_short: PrincipalId::new_anonymous().to_string(),
                    proposal_id: proposal.info.id,
                }),
            }
            return Ok(stage_actions);
        }

        stage_actions.push(SubnetAction::Noop {
            subnet_short: "unassigned-nodes".to_string(),
        });
        return Ok(stage_actions);
    }

    for subnet_short in &stage.subnets {
        // Get desired version
        let (subnet_principal, desired_version) = desired_versions
            .subnets
            .iter()
            .find(|(s, _)| s.to_string().starts_with(subnet_short))
            .expect("should find the subnet");

        // Find subnet to by the subnet_short
        let subnet = subnets
            .iter()
            .find(|s| *subnet_principal == s.principal)
            .expect("subnet should exist");

        if let Some(logger) = logger {
            debug!(
                logger,
                "Checking if subnet {} is on desired version '{}'", subnet_short, desired_version.version
            );
        }

        // If subnet is on desired version, check bake time
        if *subnet.replica_version == desired_version.version {
            let remaining =
                get_remaining_bake_time_for_subnet(last_bake_status, subnet, stage.bake_time.as_secs_f64())?;
            let remaining_duration = Duration::from_secs_f64(remaining);
            let formatted = format_duration(remaining_duration);

            if remaining != 0.0 {
                stage_actions.push(SubnetAction::Baking {
                    subnet_short: subnet_short.clone(),
                    remaining: remaining_duration,
                });
                continue;
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
            });
            continue;
        }

        // If subnet is not on desired version, check if there is an open proposal
        if let Some(proposal) = get_open_proposal_for_subnet(subnet_update_proposals, subnet, &desired_version.version)
        {
            if let Some(logger) = logger {
                info!(
                    logger,
                    "For subnet '{}' found open proposal with id '{}'", subnet_short, proposal.info.id
                )
            }
            stage_actions.push(SubnetAction::PendingProposal {
                subnet_short: subnet_short.clone(),
                proposal_id: proposal.info.id,
            });
            continue;
        }

        // If subnet is not on desired version and there is no open proposal submit it
        stage_actions.push(SubnetAction::PlaceProposal {
            is_unassigned: false,
            subnet_principal: subnet.principal,
            version: desired_version.version.clone(),
        })
    }

    Ok(stage_actions)
}

#[derive(Clone, Debug)]
pub struct DesiredReleaseVersion {
    pub subnets: BTreeMap<PrincipalId, crate::calculation::Version>,
    pub unassigned_nodes: crate::calculation::Version,
    pub release: crate::calculation::Release,
}

pub fn desired_rollout_release_version<'a>(
    subnets: &'a [Subnet],
    releases: &'a [crate::calculation::Release],
) -> DesiredReleaseVersion {
    let subnets_releases = subnets
        .iter()
        .map(|s| {
            releases
                .iter()
                .find(|r| r.versions.iter().any(|v| v.version == s.replica_version))
                .expect("version should exist in releases")
        })
        .unique()
        .collect::<Vec<_>>();
    // assumes `releases` are already sorted, but we can sort it if needed
    if subnets_releases.len() > 2 {
        panic!("more than two releases active")
    }
    let mut newest_release = releases
        .iter()
        .find(|r| subnets_releases.contains(r))
        .expect("should find some release");

    if subnets_releases.len() == 1 {
        newest_release = &releases[releases
            .iter()
            .position(|r| r == newest_release)
            .expect("release should exist")
            .saturating_sub(1)];
    }
    DesiredReleaseVersion {
        release: newest_release.clone(),
        subnets: subnets
        .iter()
        .map(|s| {
            (
                s.principal,
                newest_release
                    .versions
                    .iter()
                    .find_or_first(|v| v.subnets.iter().any(|vs| s.principal.to_string().starts_with(vs)))
                    .expect("versions should not be empty so it should return the first element if it doesn't match anything").clone(),
            )
        })
        .collect(),
         unassigned_nodes: newest_release.versions[0].clone(),
    }
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

    match bake.ge(&stage_bake_time) {
        true => Ok(0.0),
        false => {
            let remaining = Duration::from_secs_f64(stage_bake_time - bake);
            Ok(remaining.as_secs_f64())
        }
    }
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
mod week_passed_tests {
    use super::*;
    use chrono::NaiveDate;
    use rstest::rstest;

    #[rstest]
    #[case("2024-03-13", "2024-03-18", true)]
    #[case("2024-03-13", "2024-03-19", true)]
    #[case("2024-03-03", "2024-03-19", true)]
    #[case("2024-03-13", "2024-03-13", false)]
    #[case("2024-03-13", "2024-03-15", false)]
    #[case("2024-03-13", "2024-03-17", false)]
    fn should_complete(#[case] release_start: &str, #[case] now: &str, #[case] outcome: bool) {
        let release_start = NaiveDate::parse_from_str(release_start, "%Y-%m-%d").expect("Should be able to parse date");
        let now = NaiveDate::parse_from_str(now, "%Y-%m-%d").expect("Should be able to parse date");

        assert_eq!(week_passed(release_start, now), outcome)
    }
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

    pub(super) fn craft_subnet_from_id(subnet_id: &str) -> Subnet {
        Subnet {
            principal: PrincipalId(Principal::from_str(subnet_id).expect("Can create principal")),
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
            &[
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
            &[
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
mod test {

    use ic_base_types::PrincipalId;
    use ic_management_types::SubnetMetadata;
    use pretty_assertions::assert_eq;

    use crate::calculation::{Release, Version};

    use super::*;

    pub(super) fn subnet(id: u64, version: &str) -> Subnet {
        Subnet {
            principal: PrincipalId::new_subnet_test_id(id),
            replica_version: version.to_string(),
            metadata: SubnetMetadata {
                name: format!("{id}"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub(super) fn release(name: &str, versions: Vec<(&str, Vec<u64>)>) -> Release {
        Release {
            rc_name: name.to_string(),
            versions: versions
                .iter()
                .map(|(v, subnets)| Version {
                    version: v.to_string(),
                    subnets: subnets
                        .iter()
                        .map(|id| PrincipalId::new_subnet_test_id(*id).to_string())
                        .collect(),
                    ..Default::default()
                })
                .collect(),
        }
    }

    #[test]
    fn desired_version_test_cases() {
        struct TestCase {
            name: &'static str,
            subnets: Vec<Subnet>,
            releases: Vec<Release>,
            want: BTreeMap<u64, String>,
        }

        for tc in vec![
            TestCase {
                name: "all versions on the newest version already",
                subnets: vec![subnet(1, "A.default")],
                releases: vec![release("A", vec![("A.default", vec![])])],
                want: vec![(1, "A.default")]
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
            },
            TestCase {
                name: "upgrade one subnet",
                subnets: vec![subnet(1, "B.default"), subnet(2, "A.default")],
                releases: vec![
                    release("B", vec![("B.default", vec![])]),
                    release("A", vec![("A.default", vec![])]),
                ],
                want: vec![(1, "B.default"), (2, "B.default")]
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
            },
            TestCase {
                name: "extra new and old releases are ignored",
                subnets: vec![subnet(1, "C.default"), subnet(2, "B.default")],
                releases: vec![
                    release("D", vec![("D.default", vec![])]),
                    release("C", vec![("C.default", vec![])]),
                    release("B", vec![("B.default", vec![])]),
                    release("A", vec![("A.default", vec![])]),
                ],
                want: vec![(1, "C.default"), (2, "C.default")]
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
            },
            TestCase {
                name: "all subnets on same release, should proceed to upgrade everything to newer release",
                subnets: vec![subnet(1, "B.default"), subnet(2, "B.default")],
                releases: vec![
                    release("D", vec![("D.default", vec![])]),
                    release("C", vec![("C.default", vec![]), ("C.feature", vec![2])]),
                    release("B", vec![("B.default", vec![])]),
                    release("A", vec![("A.default", vec![])]),
                ],
                want: vec![(1, "C.default"), (2, "C.feature")]
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
            },
            TestCase {
                name: "feature",
                subnets: vec![subnet(1, "B.default"), subnet(2, "A.default"), subnet(3, "A.default")],
                releases: vec![
                    release("B", vec![("B.default", vec![]), ("B.feature", vec![2])]),
                    release("A", vec![("A.default", vec![])]),
                ],
                want: vec![(1, "B.default"), (2, "B.feature"), (3, "B.default")]
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
            },
        ] {
            let desired_release = desired_rollout_release_version(&tc.subnets, &tc.releases);
            assert_eq!(
                tc.want
                    .into_iter()
                    .map(|(k, v)| (PrincipalId::new_subnet_test_id(k), v))
                    .collect::<Vec<_>>(),
                desired_release
                    .subnets
                    .into_iter()
                    .map(|(k, v)| (k, v.version))
                    .collect::<Vec<_>>(),
                "test case '{}' failed",
                tc.name,
            )
        }
    }
}

// E2E tests for decision making process for happy path without feature builds
#[cfg(test)]
#[allow(dead_code)]
mod check_stages_tests {

    use check_stages_tests::test::subnet;
    use ic_base_types::PrincipalId;
    use ic_management_backend::proposal::ProposalInfoInternal;
    use registry_canister::mutations::{
        do_update_subnet_replica::UpdateSubnetReplicaVersionPayload,
        do_update_unassigned_nodes_config::UpdateUnassignedNodesConfigPayload,
    };

    use crate::calculation::{Index, Rollout};

    use self::test::release;

    use super::*;

    /// Part one => No feature builds
    /// `last_bake_status` - can be defined
    /// `subnet_update_proposals` - can be defined
    /// `unassigned_node_update_proposals` - can be defined
    /// `index` - must be defined
    /// `logger` - can be defined, but won't be because these are only tests
    /// `unassigned_version` - should be defined
    /// `subnets` - should be defined
    /// `now` - should be defined
    ///
    /// For all use cases we will use the following setup
    /// rollout:
    ///     pause: false // Tested in `should_proceed.rs` module
    ///     skip_days: [] // Tested in `should_proceed.rs` module
    ///     stages:
    ///         - subnets: [1]
    ///           bake_time: 8h
    ///         - subnets: [2, 3]
    ///           bake_time: 4h
    ///         - update_unassigned_nodes: true
    ///         - subnets: [4]
    ///           wait_for_next_week: true
    ///           bake_time: 4h
    /// releases:
    ///     - rc_name: rc--2024-02-21_23-01
    ///       versions:
    ///         - version: b
    ///           name: rc--2024-02-21_23-01
    ///           release_notes_ready: <not-important>
    ///           subnets: [] // empty because its a regular build
    ///     - rc_name: rc--2024-02-14_23-01
    ///       versions:
    ///         - version: a
    ///           name: rc--2024-02-14_23-01
    ///           release_notes_ready: <not-important>
    ///           subnets: [] // empty because its a regular build
    fn craft_index_state() -> Index {
        Index {
            rollout: Rollout {
                pause: false,
                skip_days: vec![],
                stages: vec![
                    stage(&[1], "8h"),
                    stage(&[2, 3], "4h"),
                    stage_unassigned(),
                    stage_next_week(&[4], "4h"),
                ],
            },
            releases: vec![
                release("rc--2024-02-21_23-01", vec![("b", vec![])]),
                release("rc--2024-02-14_23-01", vec![("a", vec![])]),
            ],
        }
    }

    fn stage(subnet_ids: &[u64], bake_time: &'static str) -> Stage {
        Stage {
            bake_time: humantime::parse_duration(bake_time).expect("Should be able to parse."),
            subnets: subnet_ids.iter().map(|id| principal(*id).to_string()).collect_vec(),
            ..Default::default()
        }
    }
    fn stage_unassigned() -> Stage {
        Stage {
            update_unassigned_nodes: true,
            ..Default::default()
        }
    }
    fn stage_next_week(subnet_ids: &[u64], bake_time: &'static str) -> Stage {
        let mut stage = stage(subnet_ids, bake_time);
        stage.wait_for_next_week = true;
        stage
    }

    pub(super) fn craft_subnets() -> Vec<Subnet> {
        vec![subnet(1, "a"), subnet(2, "a"), subnet(3, "a"), subnet(4, "a")]
    }

    struct TestCase {
        name: &'static str,
        index: Index,
        subnets: Vec<Subnet>,
        subnet_update_proposals: Vec<SubnetUpdateProposal>,
        unassigned_node_proposals: Vec<UpdateUnassignedNodesProposal>,
        now: NaiveDate,
        last_bake_status: BTreeMap<String, f64>,
        unassigned_node_version: String,
        expect_outcome_success: bool,
        expect_actions: Vec<SubnetAction>,
        release_start: NaiveDate,
    }

    impl Default for TestCase {
        fn default() -> Self {
            Self {
                name: Default::default(),
                index: craft_index_state(),
                subnets: craft_subnets(),
                subnet_update_proposals: Default::default(),
                unassigned_node_proposals: Default::default(),
                now: NaiveDate::parse_from_str("2024-02-26", "%Y-%m-%d").expect("Should parse date"),
                expect_actions: Default::default(),
                last_bake_status: Default::default(),
                unassigned_node_version: Default::default(),
                expect_outcome_success: true,
                release_start: NaiveDate::parse_from_str("2024-02-26", "%Y-%m-%d").expect("Should parse date"),
            }
        }
    }

    impl TestCase {
        pub fn new(name: &'static str) -> Self {
            TestCase {
                name,
                ..Default::default()
            }
        }

        pub fn with_index(mut self, index: Index) -> Self {
            self.index = index;
            self
        }

        pub fn with_subnets(mut self, subnets: &[Subnet]) -> Self {
            self.subnets = subnets.to_vec();
            self
        }

        pub fn with_subnet_update_proposals(mut self, subnet_update_proposals: &[(u64, bool, &'static str)]) -> Self {
            self.subnet_update_proposals = subnet_update_proposals
                .iter()
                .map(|(id, executed, version)| {
                    if *executed {
                        if let Some(subnet) = self
                            .subnets
                            .iter_mut()
                            .find(|subnet| subnet.principal.eq(&principal(*id)))
                        {
                            subnet.replica_version = version.to_string()
                        }
                    }
                    subnet_update_proposal(*id, *executed, version)
                })
                .collect();
            self
        }

        pub fn with_unassigned_node_proposals(
            mut self,
            unassigned_node_update_proposals: &[(bool, &'static str)],
        ) -> Self {
            self.unassigned_node_proposals = unassigned_node_update_proposals
                .iter()
                .map(|(executed, v)| {
                    if *executed {
                        self.unassigned_node_version = v.to_string()
                    }
                    UpdateUnassignedNodesProposal {
                        info: ProposalInfoInternal {
                            executed: *executed,
                            executed_timestamp_seconds: 0,
                            id: 0,
                            proposal_timestamp_seconds: 0,
                        },
                        payload: UpdateUnassignedNodesConfigPayload {
                            replica_version: Some(v.to_string()),
                            ssh_readonly_access: None,
                        },
                    }
                })
                .collect();
            self
        }

        pub fn with_now(mut self, now: &'static str) -> Self {
            self.now = NaiveDate::parse_from_str(now, "%Y-%m-%d").expect("Should parse date");
            self
        }

        pub fn expect_actions(mut self, expect_actions: &[SubnetAction]) -> Self {
            self.expect_actions = expect_actions.to_vec();
            self
        }

        pub fn with_last_bake_status(mut self, last_bake_status: &[(u64, &'static str)]) -> Self {
            self.last_bake_status = last_bake_status
                .iter()
                .map(|(id, duration)| {
                    (
                        principal(*id).to_string(),
                        humantime::parse_duration(duration)
                            .expect("Should be able to parse duration for test")
                            .as_secs_f64(),
                    )
                })
                .collect();
            self
        }

        pub fn with_unassigned_node_version(mut self, unassigned_node_version: &'static str) -> Self {
            self.unassigned_node_version = unassigned_node_version.to_string();
            self
        }

        pub fn expect_outcome_success(mut self, expect_outcome_success: bool) -> Self {
            self.expect_outcome_success = expect_outcome_success;
            self
        }

        pub fn with_release_start(mut self, release_start: &'static str) -> Self {
            self.release_start = NaiveDate::parse_from_str(release_start, "%Y-%m-%d").expect("Should parse date");
            self
        }
    }

    fn principal(id: u64) -> PrincipalId {
        PrincipalId::new_subnet_test_id(id)
    }

    fn subnet_update_proposal(subnet_id: u64, executed: bool, version: &'static str) -> SubnetUpdateProposal {
        SubnetUpdateProposal {
            info: ProposalInfoInternal {
                executed,
                executed_timestamp_seconds: 0,
                proposal_timestamp_seconds: 0,
                id: subnet_id,
            },
            payload: UpdateSubnetReplicaVersionPayload {
                replica_version_id: version.to_string(),
                subnet_id: principal(subnet_id),
            },
        }
    }

    #[test]
    fn test_use_cases_no_feature_builds() {
        let tests = vec![
            TestCase::new("Beginning of a new rollout").expect_actions(&[SubnetAction::PlaceProposal {
                is_unassigned: false,
                subnet_principal: principal(1),
                version: "b".to_string(),
            },]),
            TestCase::new("First batch is submitted but the proposal wasn't executed")
                .with_subnet_update_proposals(&[(1, false, "b"),])
                .expect_actions(&[SubnetAction::PendingProposal {
                    subnet_short: principal(1).to_string(),
                    proposal_id: 1,
                }]),
            TestCase::new("First batch is submitted the proposal was executed and the subnet is baking")
                .with_subnet_update_proposals(&[(1, true, "b"),])
                .with_last_bake_status(&[(1, "3h")])
                .expect_actions(&[SubnetAction::Baking {
                    subnet_short: principal(1).to_string(),
                    remaining: humantime::parse_duration("5h").expect("Should parse duration"),
                }]),
            TestCase::new("First batch is submitted the proposal was executed and the subnet is baked, placing proposal for next stage")
                .with_subnet_update_proposals(&[(1, true, "b")])
                .with_last_bake_status(&[(1, "9h")])
                .expect_actions(&[SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(2), version: "b".to_string() }, SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(3), version: "b".to_string() }]),
            TestCase::new("Updating unassigned nodes")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b"), (3, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "5h"), (3, "5h")])
                .with_unassigned_node_version("a")
                .expect_actions(&[SubnetAction::PlaceProposal { is_unassigned: true, subnet_principal: PrincipalId::new_anonymous(), version: "b".to_string()}]),
            TestCase::new("Proposal sent for updating unassigned nodes but it is not executed")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b"), (3, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "5h"), (3, "5h")])
                .with_unassigned_node_version("a")
                .with_unassigned_node_proposals(&[(false, "b")])
                .expect_actions(&[SubnetAction::PendingProposal { subnet_short: PrincipalId::new_anonymous().to_string(), proposal_id: 0 }]),
            TestCase::new("Executed update unassigned nodes, waiting for next week")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b"), (3, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "5h"), (3, "5h")])
                .with_unassigned_node_proposals(&[(true, "b")])
                .with_now("2024-03-03")
                .expect_actions(&[SubnetAction::WaitForNextWeek { subnet_short: principal(4).to_string() }]),
            TestCase::new("Next monday came, should place proposal for updating the last subnet")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b"), (3, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "5h"), (3, "5h")])
                .with_unassigned_node_proposals(&[(true, "b")])
                .with_now("2024-03-04")
                .expect_actions(&[SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(4), version: "b".to_string() }]),
            TestCase::new("Next monday came, proposal for last subnet executed and bake time passed. Rollout finished")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b"), (3, true, "b"), (4, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "5h"), (3, "5h"), (4, "5h")])
                .with_unassigned_node_proposals(&[(true, "b")])
                .with_now("2024-03-04")
                .expect_actions(&[]),
            TestCase::new("Partially executed step, a subnet is baking but the other doesn't have a submitted proposal")
                .with_subnet_update_proposals(&[(1, true, "b"), (2, true, "b")])
                .with_last_bake_status(&[(1, "9h"), (2, "3h")])
                .expect_actions(&[SubnetAction::Baking { subnet_short: principal(2).to_string(), remaining: humantime::parse_duration("1h").expect("Should parse duration") }, SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(3), version: "b".to_string() }])
        ];

        for test in tests {
            let desired_versions = desired_rollout_release_version(&test.subnets, &test.index.releases);
            let maybe_actions = check_stages(
                &test.last_bake_status,
                &test.subnet_update_proposals,
                &test.unassigned_node_proposals,
                test.index,
                None,
                &test.unassigned_node_version,
                &test.subnets,
                test.now,
                test.release_start,
                desired_versions,
            );

            assert_eq!(
                maybe_actions.is_ok(),
                test.expect_outcome_success,
                "test case '{}' failed",
                test.name
            );
            if !test.expect_outcome_success {
                continue;
            }

            let actions = maybe_actions.unwrap();
            assert_eq!(actions, test.expect_actions, "test case '{}' failed", test.name)
        }
    }

    #[test]
    fn test_use_cases_with_feature_builds() {
        let index_with_features = Index {
            rollout: Rollout {
                pause: false,
                skip_days: vec![],
                stages: vec![
                    stage(&[1], "8h"),
                    stage(&[2, 3], "4h"),
                    stage_unassigned(),
                    stage_next_week(&[4], "4h"),
                ],
            },
            releases: vec![
                release("rc--2024-02-21_23-01", vec![("b", vec![]), ("b.feat", vec![1, 2])]),
                release("rc--2024-02-14_23-01", vec![("a", vec![])]),
            ],
        };
        let tests = vec![TestCase::new("Beginning of a new rollout")
            .with_index(index_with_features.clone())
            .expect_actions(&[SubnetAction::PlaceProposal {
                is_unassigned: false,
                subnet_principal: principal(1),
                version: "b.feat".to_string(),
            }]), TestCase::new("First batch is submitted the proposal was executed and the subnet is baked, placing proposal for next stage")
            .with_index(index_with_features)
            .with_last_bake_status(&[(1, "9h")])
            .with_subnet_update_proposals(&[(1, true, "b.feat")])
            .expect_actions(&[SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(2), version: "b.feat".to_string() }, SubnetAction::PlaceProposal { is_unassigned: false, subnet_principal: principal(3), version: "b".to_string() }])];

        for test in tests {
            let desired_versions = desired_rollout_release_version(&test.subnets, &test.index.releases);
            let maybe_actions = check_stages(
                &test.last_bake_status,
                &test.subnet_update_proposals,
                &test.unassigned_node_proposals,
                test.index,
                None,
                &test.unassigned_node_version,
                &test.subnets,
                test.now,
                test.release_start,
                desired_versions,
            );

            assert_eq!(
                maybe_actions.is_ok(),
                test.expect_outcome_success,
                "test case '{}' failed",
                test.name
            );
            if !test.expect_outcome_success {
                continue;
            }

            let actions = maybe_actions.unwrap();
            assert_eq!(actions, test.expect_actions, "test case '{}' failed", test.name)
        }
    }
}
