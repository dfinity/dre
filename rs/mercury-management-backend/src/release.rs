use anyhow::Result;
use chrono::{Date, Datelike, NaiveTime, TimeZone, Utc, Weekday};
use ic_types::PrincipalId;
use mercury_management_types::{ReplicaRelease, Subnet};
use serde::Serialize;
use std::collections::HashMap;
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

#[derive(Default, Serialize, Clone)]
pub struct RolloutStage {
    pub start_timestamp_seconds: u64,
    pub updates: Vec<SubnetUpdate>,
    pub active: bool,
}

impl RolloutStage {
    pub fn start_date(&self) -> Date<Utc> {
        Utc.timestamp(self.start_timestamp_seconds as i64, 0).date()
    }

    pub fn new_scheduled(updates: Vec<SubnetUpdate>, day: chrono::Date<Utc>, time: NaiveTime) -> Self {
        Self {
            start_timestamp_seconds: day.and_time(time).expect("failed to compute time").timestamp() as u64,
            updates,
            active: false,
        }
    }
}

#[derive(Serialize)]
pub struct RolloutState {
    pub latest_release: ReplicaRelease,
    pub stages: Vec<RolloutStage>,
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
    pub async fn build(self) -> Result<Vec<RolloutState>> {
        const MAX_ROLLOUTS: usize = 2;

        let subnet_update_proposals = self.proposal_agent.list_update_subnet_version_proposals().await?;

        let mut rollout_states = Vec::new();
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
            rollout_states.push(state);
        }
        Ok(rollout_states)
    }

    fn get_day_stages_times(&self) -> Vec<NaiveTime> {
        vec![
            NaiveTime::from_hms_nano(7, 0, 0, 0),
            NaiveTime::from_hms_nano(9, 0, 0, 0),
            NaiveTime::from_hms_nano(11, 0, 0, 0),
            NaiveTime::from_hms_nano(13, 0, 0, 0),
        ]
    }

    async fn new_rollout_state(
        &self,
        release: ReplicaRelease,
        subnet_update_proposals: Vec<SubnetUpdateProposal>,
    ) -> Result<RolloutState> {
        let submitted_stages = self.stages_from_proposals(&release, subnet_update_proposals).await?;
        let today = Utc::now().date();
        let rollout_days = self.rollout_days(submitted_stages.first().map(|s| s.start_date()).unwrap_or(today));
        let leftover_days = rollout_days.iter().filter(|d| **d >= today).collect::<Vec<_>>();

        let mut remaining_stages = vec![];

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

        // Canary
        const CANARY_STAGE_TIME_ENTRY_INDEX: usize = 2;
        let last_stage_time = if submitted_stages.is_empty() {
            let time = self.get_day_stages_times()[CANARY_STAGE_TIME_ENTRY_INDEX];
            remaining_stages.push(RolloutStage::new_scheduled(
                // TODO: get canary ID from config (io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe)
                leftover_subnets
                    .drain(0..1)
                    .map(|s| self.create_subnet_update(s, &release))
                    .collect(),
                today,
                time,
            ));
            Some(time)
        } else {
            submitted_stages
                .last()
                .map(|s| chrono::NaiveDateTime::from_timestamp(s.start_timestamp_seconds as i64, 0).time())
        };

        // Leftover rollouts of the day
        const MIN_STAGE_SIZE: usize = 2;
        let stage_size_today = std::cmp::max(
            submitted_stages.last().map(|s| s.updates.len()).unwrap_or_default(),
            MIN_STAGE_SIZE,
        );
        for time in self
            .get_day_stages_times()
            .into_iter()
            .filter(|t| last_stage_time.map(|l| *t > l).unwrap_or(true))
        {
            remaining_stages.push(RolloutStage::new_scheduled(
                leftover_subnets
                    .drain(0..std::cmp::min(stage_size_today, leftover_subnets.len()))
                    .map(|s| self.create_subnet_update(s, &release))
                    .collect(),
                today,
                time,
            ));
        }

        // Leftover rollouts of the week
        for (i, day) in leftover_days
            .iter()
            .skip(1)
            .enumerate()
            .take(leftover_days.len().saturating_sub(2))
        {
            for (j, time) in self.get_day_stages_times().into_iter().enumerate() {
                // Don't increase the size of the first stage compared to previous day
                let stage_size = stage_size_today + i + std::cmp::min(j, 1);
                remaining_stages.push(RolloutStage::new_scheduled(
                    leftover_subnets
                        .drain(0..std::cmp::min(stage_size, leftover_subnets.len()))
                        .map(|s| self.create_subnet_update(s, &release))
                        .collect(),
                    **day,
                    time,
                ));
            }
        }

        // Fill in the remaining rollouts starting from the last stage
        for (i, s) in leftover_subnets.into_iter().enumerate() {
            let remaining_stages_count = remaining_stages.len();
            remaining_stages[remaining_stages_count - 1 - (i % remaining_stages_count)]
                .updates
                .push(self.create_subnet_update(s, &release));
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
                remaining_stages.push(RolloutStage::new_scheduled(
                    vec![self.create_subnet_update(nns_subnet, &release)],
                    **last_day,
                    self.get_day_stages_times()[0],
                ))
            }
        }

        Ok(RolloutState {
            latest_release: release,
            stages: submitted_stages
                .into_iter()
                .chain(remaining_stages.into_iter())
                .filter(|s| !s.updates.is_empty())
                .collect(),
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
                    acc.push(RolloutStage {
                        start_timestamp_seconds: proposal.info.proposal_timestamp_seconds,
                        updates: vec![update],
                        active: true,
                    })
                }
            }
            acc
        });

        if let Some(last_stage) = stages.last_mut() {
            let update_states = self
                .get_update_states(
                    release,
                    chrono::DateTime::<Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(last_stage.start_timestamp_seconds as i64, 0),
                        Utc,
                    ),
                )
                .await?;
            for u in &mut last_stage.updates {
                if matches!(u.state, SubnetUpdateState::Unknown) {
                    if let Some(state) = update_states.get(&u.subnet_id) {
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
            .split_inclusive(|p| p.iso_week().week() > start.iso_week().week() && *p >= Utc::now().date())
            .next()
            .unwrap()
            .to_vec()
        // TODO: exclude days based on config
    }
}
