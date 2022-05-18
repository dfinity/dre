use anyhow::Result;
use ic_nns_governance::pb::v1::{ProposalInfo, ProposalStatus};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use mercury_management_types::{ReplicaRelease, Subnet};
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;

use crate::prom::SubnetUpgrade;

#[derive(Clone, Serialize)]
pub struct Rollout {
    pub stages: Vec<RolloutStage>,
    pub release: ReplicaRelease,
}

#[derive(Clone, Serialize)]
pub struct RolloutStage {
    name: String,
    subnets: Vec<SubnetRolloutStatus>,
}

#[derive(Clone, Serialize)]
pub struct SubnetRolloutStatus {
    principal: PrincipalId,
    latest_release: bool,
    upgrading: bool,
    upgrading_release: bool,
    replica_release: ReplicaRelease,
    patches_available: Vec<ReplicaRelease>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    proposal: Option<ReplicaVersionUpdateProposal>,
}

#[derive(Clone, Serialize)]
pub struct ReplicaVersionUpdateProposal {
    pub id: u64,
    pub pending: bool,
}

impl Rollout {
    pub fn new(
        subnets: HashMap<PrincipalId, Subnet>,
        proposals: Vec<(ProposalInfo, UpdateSubnetReplicaVersionPayload)>,
        mut subnets_upgrades: HashMap<PrincipalId, SubnetUpgrade>,
        releases: Vec<ReplicaRelease>,
    ) -> Result<Self> {
        let mut unused_subnets: Vec<Subnet> = vec![];
        let mut verified_subnets: Vec<Subnet> = vec![];
        let mut canary_subnets: Vec<Subnet> = vec![];
        let mut ga_subnets: Vec<Subnet> = vec![];
        let mut system_subnets: Vec<Subnet> = vec![];

        let unused_subnets_principals = vec![
            PrincipalId::from_str("w4asl-4nmyj-qnr7c-6cqq4-tkwmt-o26di-iupkq-vx4kt-asbrx-jzuxh-4ae")
                .expect("invalid principal"),
            PrincipalId::from_str("io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe")
                .expect("invalid principal"),
            PrincipalId::from_str("qxesv-zoxpm-vc64m-zxguk-5sj74-35vrb-tbgwg-pcird-5gr26-62oxl-cae")
                .expect("invalid principal"),
            PrincipalId::from_str("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae")
                .expect("invalid principal"),
        ];
        let canary_subnets_principals = vec![
            PrincipalId::from_str("shefu-t3kr5-t5q3w-mqmdq-jabyv-vyvtf-cyyey-3kmo4-toyln-emubw-4qe")
                .expect("invalid principal"),
            PrincipalId::from_str("pjljw-kztyl-46ud4-ofrj6-nzkhm-3n4nt-wi3jt-ypmav-ijqkt-gjf66-uae")
                .expect("invalid principal"),
            PrincipalId::from_str("gmq5v-hbozq-uui6y-o55wc-ihop3-562wb-3qspg-nnijg-npqp5-he3cj-3ae")
                .expect("invalid principal"),
        ];

        let rollout_release_name = releases
            .last()
            .ok_or_else(|| anyhow::format_err!("no releases"))?
            .name
            .clone();
        let rollout_releases: Vec<_> = releases.iter().skip_while(|r| r.name != rollout_release_name).collect();

        let mut subnets_sorted = subnets.into_values().collect::<Vec<_>>();
        subnets_sorted.sort_by(|a, b| {
            let subnet_a_rollout_index = rollout_releases
                .iter()
                .position(|r| a.replica_version == r.commit_hash)
                .map(|p| p as i32)
                .unwrap_or(-1);
            let subnet_b_rollout_index = rollout_releases
                .iter()
                .position(|r| b.replica_version == r.commit_hash)
                .map(|p| p as i32)
                .unwrap_or(-1);
            subnet_b_rollout_index
                .cmp(&subnet_a_rollout_index)
                .then(a.principal.cmp(&b.principal))
        });

        for s in subnets_sorted {
            if s.subnet_type == SubnetType::System {
                system_subnets.push(s);
            } else if unused_subnets_principals.contains(&s.principal) {
                unused_subnets.push(s);
            } else if canary_subnets_principals.contains(&s.principal) {
                canary_subnets.push(s);
            } else if s.subnet_type == SubnetType::VerifiedApplication {
                verified_subnets.push(s);
            } else if s.subnet_type == SubnetType::Application {
                ga_subnets.push(s);
            } else {
                return Err(anyhow::format_err!(
                    "failed to assign the subnet {} to a group",
                    s.principal
                ));
            }
        }
        canary_subnets.sort_by(|a, b| {
            let a_index = canary_subnets_principals
                .iter()
                .position(|p| *p == a.principal)
                .expect("unable to find subnet index");
            let b_index = canary_subnets_principals
                .iter()
                .position(|p| *p == b.principal)
                .expect("unable to find subnet index");
            a_index.cmp(&b_index)
        });

        let update_version_proposals = proposals
            .into_iter()
            .filter(|(_, p)| rollout_releases.iter().any(|r| p.replica_version_id == r.commit_hash))
            .collect::<Vec<_>>();

        Ok(Self {
            release: (*rollout_releases.last().expect("no releases")).clone(),
            stages: [
                vec![("Unused", vec![unused_subnets[0].clone()])],
                unused_subnets[1..]
                    .iter()
                    .cloned()
                    .zip(canary_subnets.into_iter())
                    .map(|s| ("Canary & Unused", [s.0, s.1].to_vec()))
                    .collect(),
                verified_subnets
                    .into_iter()
                    .chain(ga_subnets.into_iter())
                    .enumerate()
                    .group_by(|(i, _)| i / 2)
                    .into_iter()
                    .map(|(_, g)| ("GA & Verified", g.into_iter().map(|(_, s)| s).collect::<Vec<_>>()))
                    .collect(),
                vec![("System", system_subnets)],
            ]
            .concat()
            .into_iter()
            .map(|(name, subnets)| RolloutStage {
                name: name.to_string(),
                subnets: subnets
                    .into_iter()
                    .map(|s| {
                        let proposal = update_version_proposals.iter().find(|(proposal, p)| {
                            p.subnet_id == s.principal
                                && ProposalStatus::from_i32(proposal.status).expect("unknown proposal status")
                                    != ProposalStatus::Rejected
                        });
                        let replica_release = releases
                            .iter()
                            .find(|r| r.commit_hash == s.replica_version)
                            .expect("release not found")
                            .clone();

                        let subnet_upgrade = subnets_upgrades.remove(&s.principal).unwrap_or(SubnetUpgrade {
                            subnet_principal: s.principal,
                            old_version: s.replica_version.clone(),
                            new_version: s.replica_version.clone(),
                            upgraded: false,
                        });

                        SubnetRolloutStatus {
                            principal: s.principal,
                            latest_release: rollout_releases.iter().any(|r| r.commit_hash == s.replica_version),
                            upgrading: !subnet_upgrade.upgraded,
                            upgrading_release: !subnet_upgrade.upgraded
                                && rollout_releases
                                    .iter()
                                    .all(|r| r.commit_hash != subnet_upgrade.old_version),
                            patches_available: releases
                                .iter()
                                .filter(|r| r.name == replica_release.name && r.patches(&replica_release))
                                .cloned()
                                .collect(),
                            replica_release,
                            name: s.metadata.name.clone(),
                            proposal: proposal.and_then(|(p, _)| p.id.map(|id| (id.id, p))).map(|(id, p)| {
                                ReplicaVersionUpdateProposal {
                                    id,
                                    pending: ProposalStatus::from_i32(p.status).unwrap() == ProposalStatus::Open,
                                }
                            }),
                        }
                    })
                    .collect(),
            })
            .collect(),
        })
    }
}
