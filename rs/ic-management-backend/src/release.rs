use anyhow::Result;
use chrono::serde::ts_seconds;
use chrono::{Date, Datelike, NaiveTime, TimeZone, Utc, Weekday};
use ic_management_types::{ReplicaRelease, Subnet};
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

use crate::proposal::{ProposalAgent, SubnetUpdateProposal};
use crate::registry::NNS_SUBNET_NAME;

#[derive(Serialize, Clone, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SubnetUpdateState {
    Scheduled,
    Submitted,
    Preparing,
    Updating,
    Baking,
    Complete,
    Unknown,
}

#[derive(Serialize, Clone)]
pub struct SubnetUpdate {
    pub state: SubnetUpdateState,
    pub subnet_id: PrincipalId,
    pub subnet_name: String,
    pub proposal: Option<SubnetUpdateProposal>,
    pub patches_available: Vec<ReplicaRelease>,
    pub replica_release: ReplicaRelease,
}

#[derive(Serialize, Clone)]
pub struct RolloutStage {
    #[serde(with = "ts_seconds")]
    pub start_date_time: chrono::DateTime<Utc>,
    pub start_time: Option<chrono::NaiveTime>,
    pub updates: Vec<SubnetUpdate>,
    pub active: bool,
}

impl RolloutStage {
    pub fn new_submitted(updates: Vec<SubnetUpdate>, submitted_timestamp_seconds: u64) -> Self {
        let datetime = Utc.timestamp(submitted_timestamp_seconds as i64, 0);
        Self {
            start_date_time: datetime,
            start_time: datetime.time().into(),
            updates,
            active: true,
        }
    }

    pub fn new_scheduled(updates: Vec<SubnetUpdate>, day: chrono::Date<Utc>) -> Self {
        Self {
            start_date_time: day
                .and_time(NaiveTime::from_hms(0, 0, 0))
                .expect("failed to compute time"),
            start_time: None,
            updates,
            active: false,
        }
    }
}

#[derive(Serialize)]
pub struct Rollout {
    pub state: RolloutState,
    pub latest_release: ReplicaRelease,
    pub stages: Vec<RolloutStage>,
}

#[derive(Serialize, Clone, Display, EnumString, PartialEq, Eq, Ord, PartialOrd)]
pub enum RolloutState {
    Active,
    Scheduled,
    Complete,
}

pub struct RolloutConfig {}

pub struct RolloutBuilder {
    pub config: RolloutConfig,
    pub proposal_agent: ProposalAgent,
    pub prometheus_client: prometheus_http_query::Client,
    pub subnets: HashMap<PrincipalId, Subnet>,
    pub releases: Vec<ReplicaRelease>,
    pub network: String,
}

impl RolloutBuilder {
    pub async fn build(self) -> Result<Vec<Rollout>> {
        const MAX_ROLLOUTS: usize = 1;

        let subnet_update_proposals = self.proposal_agent.list_update_subnet_version_proposals().await?;

        let mut rollouts = Vec::new();
        for r in self
            .releases
            .clone()
            .into_iter()
            .rev()
            .fold(vec![], |mut acc, r| {
                if acc.len() < MAX_ROLLOUTS && acc.last().map(|l: &ReplicaRelease| l.name != r.name).unwrap_or(true) {
                    acc.push(r);
                }
                acc
            })
            .into_iter()
            .rev()
        {
            let state = self
                .new_rollout_state(
                    r.clone(),
                    subnet_update_proposals
                        .iter()
                        .filter(|p| r.contains_patch(&p.payload.replica_version_id))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
                .await?;
            rollouts.push(state);
        }
        rollouts.sort_by_key(|a| a.state.clone());
        Ok(rollouts)
    }

    async fn new_rollout_state(
        &self,
        release: ReplicaRelease,
        subnet_update_proposals: Vec<SubnetUpdateProposal>,
    ) -> Result<Rollout> {
        let submitted_stages = self.stages_from_proposals(&release, subnet_update_proposals).await?;
        let today = Utc::now().date();
        let rollout_days = self.rollout_days(
            submitted_stages
                .first()
                .map(|s| s.start_date_time.date())
                .unwrap_or(today),
        );

        let mut leftover_subnets = self
            .subnets
            .values()
            .filter(|s| {
                submitted_stages
                    .iter()
                    .all(|stage| stage.updates.iter().all(|u| u.subnet_id != s.principal))
                    && s.metadata.name != NNS_SUBNET_NAME
            })
            .collect::<Vec<_>>();

        let mut leftover_days = rollout_days.into_iter().filter(|d| *d >= today).collect::<Vec<_>>();
        // If only the day of NNS subnet rollout is left, take just the first remaining rollout day
        if leftover_subnets.len() == 1
            && submitted_stages
                .iter()
                .all(|s| s.start_date_time.date() != leftover_days[0])
        {
            leftover_days = vec![leftover_days[0]];
        }

        let mut remaining_stages_groups = leftover_days.iter().map(|_| vec![]).collect::<Vec<Vec<RolloutStage>>>();
        let mut stage_groups_overheads = leftover_days.iter().map(|_| 0).collect::<Vec<_>>();
        // Last stage will have 2 stages less than every other to finish rollout earlier in the day.
        stage_groups_overheads[leftover_days.len().saturating_sub(2)] = 2;

        // If we're on the first day of the rollout
        if submitted_stages.iter().all(|s| s.start_date_time.date() == today) {
            // Set overhead of day group to 1
            stage_groups_overheads[0] = 1;
        }

        // Adjust the overhead for today
        stage_groups_overheads[0] += submitted_stages
            .iter()
            .filter(|s| s.start_date_time.date() == today)
            .count();

        // Add canary rollout stage if rollout didn't start yet
        if submitted_stages.is_empty() {
            remaining_stages_groups[0].push(RolloutStage::new_scheduled(
                // TODO: get canary ID from config (io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe)
                leftover_subnets
                    .drain(0..1)
                    .map(|s| self.create_subnet_update(s, &release))
                    .collect(),
                today,
            ));
        }

        let first_stage_size_today = submitted_stages
            .iter()
            .map(|s| s.start_date_time.date())
            .filter(|d| *d != today)
            .unique()
            .count()
            + 1;

        const MAX_ROLLOUT_STAGE_SIZE: usize = 4;
        while !leftover_subnets.is_empty() {
            let min_day_stages_count = remaining_stages_groups
                .iter()
                .take(remaining_stages_groups.len().saturating_sub(1))
                .enumerate()
                .map(|(i, g)| g.len() + stage_groups_overheads[i])
                .min()
                .unwrap_or_default();
            for (i, day) in leftover_days
                .iter()
                .enumerate()
                .take(leftover_days.len().saturating_sub(1))
                .rev()
            {
                // If rollout day has more stages than others (including overhead), skip
                if stage_groups_overheads[i] + remaining_stages_groups[i].len() > min_day_stages_count {
                    continue;
                }

                let stage_size = std::cmp::min(
                    MAX_ROLLOUT_STAGE_SIZE,
                    first_stage_size_today + i + std::cmp::min(remaining_stages_groups[i].len(), 1),
                );
                remaining_stages_groups[i].push(RolloutStage::new_scheduled(
                    leftover_subnets
                        .drain(0..std::cmp::min(stage_size, leftover_subnets.len()))
                        .map(|s| self.create_subnet_update(s, &release))
                        .collect(),
                    *day,
                ));
            }
        }

        // NNS Rollout
        if let Some(last_day) = leftover_days.last() {
            let nns_subnet = self
                .subnets
                .values()
                .find(|s| s.metadata.name == NNS_SUBNET_NAME)
                .expect("No NNS subnet");
            if !submitted_stages
                .iter()
                .any(|s| s.updates.iter().any(|u| u.subnet_id == nns_subnet.principal))
            {
                remaining_stages_groups.push(vec![RolloutStage::new_scheduled(
                    vec![self.create_subnet_update(nns_subnet, &release)],
                    *last_day,
                )])
            }
        }

        let stages = submitted_stages
            .into_iter()
            .chain(remaining_stages_groups.into_iter().flatten())
            .filter(|s| !s.updates.is_empty())
            .collect::<Vec<_>>();
        Ok(Rollout {
            latest_release: release,
            state: if stages.iter().all(|s| {
                s.updates
                    .iter()
                    .all(|u| matches!(u.state, SubnetUpdateState::Scheduled))
            }) {
                RolloutState::Scheduled
            } else if stages
                .iter()
                .all(|s| s.updates.iter().all(|u| matches!(u.state, SubnetUpdateState::Complete)))
            {
                RolloutState::Complete
            } else {
                RolloutState::Active
            },
            stages,
        })
    }

    fn create_subnet_update(&self, s: &Subnet, release: &ReplicaRelease) -> SubnetUpdate {
        SubnetUpdate {
            state: SubnetUpdateState::Scheduled,
            subnet_id: s.principal,
            subnet_name: s.metadata.name.clone(),
            proposal: None,
            patches_available: release.patches_for(&s.replica_version).unwrap_or_default(),
            replica_release: self
                .releases
                .iter()
                .find(|r| r.commit_hash == s.replica_version)
                .expect("version not found")
                .clone(),
        }
    }

    async fn get_update_states(
        &self,
        release: &ReplicaRelease,
        since: chrono::DateTime<Utc>,
    ) -> Result<HashMap<PrincipalId, SubnetUpdateState>> {
        const STATE_FIELD: &str = "state";
        let query = format!(
            r#"
            # Get all subnets that are not yet updated to the given release. These are preparing a CUP for the update.
            label_replace(
                sum by (ic_subnet) (ic_replica_info{{ic="{network}", ic_active_version!="{version}"}})
                    /
                max by (ic_subnet) (consensus_dkg_current_committee_size{{ic="{network}"}})
            , 
                "{state_field}", "{preparing_state}", "", ""
            )
                or ignoring({state_field})
            # Get all subnets that are running on the given release but some nodes are not up yet. These are probably restarting to do an update.
            label_replace(
                sum by (ic_subnet) (ic_replica_info{{ic="{network}", ic_active_version="{version}"}})
                    <
                max by (ic_subnet) (consensus_dkg_current_committee_size{{ic="{network}"}})
            ,
                "{state_field}", "{updating_state}", "", ""
            )
                or ignoring({state_field})
            # Get all subnets that have been running on the given release for at least half an hour without any restarts or pages since the specified time.
            # If the result is 1, the subnet completed the bake process successfully.
            label_replace(
                max_over_time((
                    -sum_over_time(
                        (sum by (ic_subnet) (ALERTS{{ic="{network}", severity="page", alertstate="firing"}}))[30m:1m]
                    )
                        or
                    (
                        avg_over_time((sum by (ic_subnet) (ic_replica_info{{ic="{network}", ic_active_version="{version}"}}))[30m:1m])
                                /
                        max_over_time((max by (ic_subnet) (consensus_dkg_current_committee_size{{ic="{network}"}}))[30m:5m])
                    )
                )[{period}s:1m])
            ,
                "{state_field}", "{baking_state}", "", ""
            )
        "#,
            network = self.network,
            version = release.commit_hash,
            preparing_state = SubnetUpdateState::Preparing,
            updating_state = SubnetUpdateState::Updating,
            baking_state = SubnetUpdateState::Baking,
            state_field = STATE_FIELD,
            period = Utc::now().timestamp() - since.timestamp(),
        );
        info!("release ({}) query: {}", release.commit_hash, query);
        let response = self.prometheus_client.query(query, None, None).await?;
        let results = response.as_instant().expect("Expected instant vector");
        Ok(results
            .iter()
            .filter_map(|r| {
                let subnet = r
                    .metric()
                    .get("ic_subnet")
                    .map(|s| PrincipalId::from_str(s).expect("ic_subnet label should always be a valid principal id"));
                let state = SubnetUpdateState::from_str(
                    r.metric()
                        .get(STATE_FIELD)
                        .expect("query should always yield a vector with a valid state"),
                )
                .expect("state label should always be a valid state");
                subnet.map(|s| {
                    (
                        s,
                        if matches!(state, SubnetUpdateState::Baking) && r.sample().value() == 1.0 {
                            SubnetUpdateState::Complete
                        } else {
                            state
                        },
                    )
                })
            })
            .collect())
    }

    async fn stages_from_proposals(
        &self,
        release: &ReplicaRelease,
        subnet_update_proposals: Vec<SubnetUpdateProposal>,
    ) -> Result<Vec<RolloutStage>> {
        const ROLLOUT_BATCH_PROPOSAL_LAG_TOLERANCE_SECONDS: u64 = 60 * 30;

        let mut proposals = subnet_update_proposals;
        proposals.sort_by(|a, b| {
            a.info
                .proposal_timestamp_seconds
                .cmp(&b.info.proposal_timestamp_seconds)
        });
        let mut stages: Vec<RolloutStage> = proposals.into_iter().fold(vec![], |mut acc, proposal| {
            let update = SubnetUpdate {
                proposal: proposal.clone().into(),
                state: if proposal.info.executed {
                    SubnetUpdateState::Unknown
                } else {
                    SubnetUpdateState::Submitted
                },
                subnet_id: proposal.payload.subnet_id,
                subnet_name: self
                    .subnets
                    .get(&proposal.payload.subnet_id)
                    .expect("missing subnet")
                    .metadata
                    .name
                    .clone(),
                patches_available: release
                    .patches_for(&proposal.payload.replica_version_id)
                    .expect("missing version"),
                replica_release: release
                    .get(&proposal.payload.replica_version_id)
                    .expect("missing version"),
            };
            match acc.last_mut() {
                Some(stage)
                    if stage
                        .updates
                        .last()
                        .map(|last| match &last.proposal {
                            Some(last_update_proposal) => {
                                proposal.info.proposal_timestamp_seconds
                                    - last_update_proposal.info.proposal_timestamp_seconds
                                    < ROLLOUT_BATCH_PROPOSAL_LAG_TOLERANCE_SECONDS
                            }
                            _ => unreachable!(),
                        })
                        .unwrap_or(true) =>
                {
                    stage.updates.push(update)
                }
                _ => {
                    if let Some(last_stage) = acc.last_mut() {
                        last_stage.active = false;
                        for u in &mut last_stage.updates {
                            u.state = SubnetUpdateState::Complete;
                        }
                    }
                    acc.push(RolloutStage::new_submitted(
                        vec![update],
                        proposal.info.proposal_timestamp_seconds,
                    ))
                }
            }
            acc
        });

        if let Some(last_stage) = stages.last_mut() {
            let releases = last_stage
                .updates
                .iter()
                .map(|u| u.replica_release.clone())
                .collect::<HashSet<_>>();
            let mut update_states = HashMap::new();
            for release in releases {
                let release_update_states = self.get_update_states(&release, last_stage.start_date_time).await?;
                update_states.insert(release, release_update_states);
            }
            for u in &mut last_stage.updates {
                if matches!(u.state, SubnetUpdateState::Unknown) {
                    if let Some(state) = update_states
                        .get(&u.replica_release)
                        .expect("should contain update states for the release")
                        .get(&u.subnet_id)
                    {
                        u.state = state.clone();
                    }
                }
            }
            last_stage.active = last_stage
                .updates
                .iter()
                .any(|u| !matches!(u.state, SubnetUpdateState::Complete));
        }
        Ok(stages)
    }

    fn rollout_days(&self, start: Date<Utc>) -> Vec<Date<Utc>> {
        let candidates = (0..14)
            .into_iter()
            .map(|i| {
                let mut d = start;
                for _ in 0..i {
                    d = d.succ();
                }
                d
            })
            .filter(|d| d.weekday().number_from_monday() < Weekday::Sat.number_from_monday())
            .collect::<Vec<_>>();
        candidates
            .split_inclusive(|p| p.iso_week().week() > start.iso_week().week() && *p > Utc::now().date())
            .next()
            .unwrap()
            .to_vec()
        // TODO: exclude days based on config
    }
}
