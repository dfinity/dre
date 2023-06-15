use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;
use chrono::serde::ts_seconds;
use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use futures_util::future::try_join_all;
use ic_management_types::{Network, ReplicaRelease, Subnet};
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
        let datetime = Utc.timestamp_opt(submitted_timestamp_seconds as i64, 0).unwrap();
        Self {
            start_date_time: datetime,
            start_time: datetime.time().into(),
            updates,
            active: true,
        }
    }

    pub fn new_scheduled(updates: Vec<SubnetUpdate>, day: NaiveDate) -> Self {
        Self {
            start_date_time: Utc
                .timestamp_opt(
                    day.and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("failed to create naive time"))
                        .timestamp(),
                    0,
                )
                .unwrap(),
            start_time: None,
            updates,
            active: false,
        }
    }
}

#[derive(Serialize)]
pub struct Rollout {
    pub status: RolloutStatus,
    pub latest_release: ReplicaRelease,
    pub stages: Vec<RolloutStage>,
}

#[derive(Serialize, Clone, Display, EnumString, PartialEq, Eq, Ord, PartialOrd)]
pub enum RolloutStatus {
    Active,
    Scheduled,
    Complete,
}

#[derive(Deserialize)]
pub struct RolloutConfig {
    #[serde(with = "iso_8601_date_format")]
    pub exclude_days: Vec<NaiveDate>,
}

mod iso_8601_date_format {
    use chrono::{NaiveDate, TimeZone, Utc};
    use itertools::Itertools;
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::deserialize(deserializer)?
            .into_iter()
            .map(|s: String| {
                Utc.datetime_from_str(&format!("{} 00:00:00", s), FORMAT)
                    .map_err(serde::de::Error::custom)
                    .map(|dt| dt.date_naive())
            })
            .try_collect()
    }
}

impl RolloutConfig {
    // Returns available rollout days. Always return at least 2 available days
    fn rollout_days(&self, start: NaiveDate) -> Vec<NaiveDate> {
        let candidates = (0..14)
            .into_iter()
            .map(|i| {
                let mut d = start;
                for _ in 0..i {
                    d = d.succ_opt().unwrap();
                }
                d
            })
            .filter(|d| d.weekday().number_from_monday() < Weekday::Sat.number_from_monday())
            .filter(|d| !self.exclude_days.contains(d))
            .collect::<Vec<_>>();
        candidates
            .split_inclusive(|p| p.iso_week().week() > start.iso_week().week() && *p > Utc::now().date_naive())
            .next()
            .unwrap()
            .to_vec()
    }
}

pub struct RolloutBuilder {
    pub proposal_agent: ProposalAgent,
    pub prometheus_client: prometheus_http_query::Client,
    pub subnets: BTreeMap<PrincipalId, Subnet>,
    pub releases: Vec<ReplicaRelease>,
    pub network: Network,
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
                .new_for_release(
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
        rollouts.sort_by_key(|a| a.status.clone());
        Ok(rollouts)
    }

    async fn new_for_release(
        &self,
        release: ReplicaRelease,
        subnet_update_proposals: Vec<SubnetUpdateProposal>,
    ) -> Result<Rollout> {
        let config: RolloutConfig =
            serde_yaml::from_slice(include_bytes!("../config/releases/default.yaml")).expect("invalid config");
        let submitted_stages = self.stages_from_proposals(&release, subnet_update_proposals).await?;
        let today = Utc::now().date_naive();
        let rollout_days = config.rollout_days(
            submitted_stages
                .first()
                .map(|s| s.start_date_time.date_naive())
                .unwrap_or(today),
        );

        let schedule = RolloutState {
            nns_subnet: self
                .subnets
                .values()
                .find(|s| s.metadata.name == NNS_SUBNET_NAME)
                .expect("NNS subnet should exist")
                .principal,
            subnets: self.subnets.values().map(|s| s.principal).collect(),
            rollout_days: rollout_days
                .iter()
                .map(|d| RolloutDay {
                    date: *d,
                    rollout_stages_subnets: submitted_stages
                        .iter()
                        .filter(|rs| rs.start_date_time.date_naive() == *d)
                        .map(|rs| rs.updates.iter().map(|u| u.subnet_id).collect())
                        .collect(),
                })
                .collect(),
            date: today,
        }
        .schedule();

        let stages = submitted_stages
            .into_iter()
            .chain(schedule.into_iter().flat_map(|rd| {
                rd.rollout_stages_subnets
                    .iter()
                    .map(|subnets| {
                        RolloutStage::new_scheduled(
                            subnets
                                .iter()
                                .map(|s| {
                                    self.create_subnet_update(
                                        self.subnets.get(s).expect("subnet should exist"),
                                        &release,
                                    )
                                })
                                .collect(),
                            rd.date,
                        )
                    })
                    .collect::<Vec<_>>()
            }))
            .filter(|s| !s.updates.is_empty())
            .collect::<Vec<_>>();

        Ok(Rollout {
            latest_release: release,
            status: if stages.iter().all(|s| {
                s.updates
                    .iter()
                    .all(|u| matches!(u.state, SubnetUpdateState::Scheduled))
            }) {
                RolloutStatus::Scheduled
            } else if stages
                .iter()
                .all(|s| s.updates.iter().all(|u| matches!(u.state, SubnetUpdateState::Complete)))
            {
                RolloutStatus::Complete
            } else {
                RolloutStatus::Active
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
                .collect::<BTreeSet<_>>();
            let mut update_states = BTreeMap::new();
            for release in releases {
                let release_update_states = get_update_states(
                    &self.network,
                    &self.prometheus_client,
                    &release,
                    last_stage.start_date_time,
                )
                .await?;
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
}

pub struct RolloutState {
    pub nns_subnet: PrincipalId,
    pub subnets: Vec<PrincipalId>,
    pub rollout_days: Vec<RolloutDay>,
    pub date: NaiveDate,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RolloutDay {
    pub date: NaiveDate,
    pub rollout_stages_subnets: Vec<Vec<PrincipalId>>,
}

impl RolloutState {
    pub fn schedule(&self) -> Vec<RolloutDay> {
        let mut remaining_rollout_days = self
            .rollout_days
            .clone()
            .into_iter()
            .filter(|rd| rd.date >= self.date)
            .collect::<Vec<_>>()
            // Exclude NNS rollout day
            .split_last()
            .expect("should contain at least two days")
            .1
            .to_vec();

        let mut leftover_subnets = self
            .subnets
            .iter()
            .filter(|s| {
                **s != self.nns_subnet
                    && !self
                        .rollout_days
                        .iter()
                        .flat_map(|rd| rd.rollout_stages_subnets.iter().flatten())
                        .contains(*s)
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut remaining_rollout_days_overheads = remaining_rollout_days.iter().map(|_| 0_usize).collect::<Vec<_>>();
        if leftover_subnets.is_empty() {
            // No app subnet rollouts are left
            remaining_rollout_days = vec![];
            remaining_rollout_days_overheads = vec![];
        } else {
            // Increase overhead for last day of app subnet rollout
            *remaining_rollout_days_overheads
                .last_mut()
                .expect("should contain at least one day") += 2;

            // Clear submitted subnets and set overhead instead by the number of already submitted stages
            remaining_rollout_days_overheads[0] += remaining_rollout_days[0].rollout_stages_subnets.len();
            remaining_rollout_days[0].rollout_stages_subnets = vec![];
        }

        // Set overhead to 1 for the first day of the rollout since previous rollout usually finishes on the same day
        if self
            .rollout_days
            .iter()
            // Find the the first rollout day
            .find(|rd| !rd.rollout_stages_subnets.is_empty())
            // Check if it's the same day as the first day in the schedule
            .map(|rd| {
                remaining_rollout_days
                    .first()
                    .map(|first| rd.date == first.date)
                    // If remaining rollout days are empty, then the app subnets rollout is finished
                    .unwrap_or(false)
            })
            // If no day is found, then the rollout didn't start yet
            .unwrap_or(true)
        {
            remaining_rollout_days_overheads[0] += 1;
        }

        // Number of the subnets rolled out for the first stage of the current date of the rollout
        let current_day_first_stage_size = self
            .rollout_days
            .iter()
            .filter(|rd| !rd.rollout_stages_subnets.is_empty())
            .map(|s| s.date)
            .filter(|d| *d != self.date)
            .unique()
            .count()
            + 1;

        const MAX_ROLLOUT_STAGE_SIZE: usize = 4;
        let mut rollout_stages_sizes: Vec<Vec<usize>> =
            remaining_rollout_days.iter().map(|_| vec![]).collect::<Vec<_>>();
        let mut leftover_subnets_count = leftover_subnets.len();
        while leftover_subnets_count != 0 {
            let min_day_stages_count = rollout_stages_sizes
                .iter()
                .enumerate()
                .map(|(i, day)| day.len() + remaining_rollout_days_overheads[i])
                .min()
                .unwrap_or_default();
            for (i, day) in remaining_rollout_days.iter().enumerate().rev() {
                // If rollout day has more stages than others (including overhead), skip
                if remaining_rollout_days_overheads[i] + rollout_stages_sizes[i].len() > min_day_stages_count {
                    continue;
                }

                if leftover_subnets_count == 0 {
                    break;
                }

                let stage_size = std::cmp::min(
                    std::cmp::min(
                        // Limit the stage size to the number of
                        MAX_ROLLOUT_STAGE_SIZE,
                        current_day_first_stage_size + i
                        // Add 1 if not first stage of the day 
                        + std::cmp::min(self.rollout_days.iter().find(|rd| rd.date == day.date).expect("rollout day should exist").rollout_stages_subnets.len() + rollout_stages_sizes[i].len(), 1),
                    ),
                    // Limit the stage size to the number of available subnets
                    leftover_subnets_count,
                );
                leftover_subnets_count -= stage_size;
                rollout_stages_sizes[i].push(stage_size)
            }
        }

        for (i, sizes) in rollout_stages_sizes.iter().enumerate() {
            let mut rollout_date = remaining_rollout_days[0].date;
            for _ in 0..i {
                rollout_date = rollout_date.succ_opt().unwrap();
            }
            remaining_rollout_days.push(RolloutDay {
                date: rollout_date,
                rollout_stages_subnets: sizes
                    .iter()
                    .map(|size| leftover_subnets.drain(0..*size).collect())
                    .collect(),
            });
        }

        // NNS Rollout
        if !self
            .rollout_days
            .iter()
            .flat_map(|rd| rd.rollout_stages_subnets.iter().flatten())
            .contains(&self.nns_subnet)
        {
            // If there's already a rollout submitted or sheduled on the current date of the rollout
            // submit the NNS proposal on the very last day of available dates
            let last_day = if self
                .rollout_days
                .iter()
                .chain(remaining_rollout_days.iter())
                .rev()
                .find(|rd| !rd.rollout_stages_subnets.is_empty())
                .map(|rd| rd.date >= self.date)
                .unwrap_or(true)
            {
                self.rollout_days.last().expect("should have at least one day").date
            // Otherwise, rollout should have 2 complete empty days available left, and then use the first
            // one and drop the last one
            } else {
                self.rollout_days
                    .iter()
                    .rev()
                    .nth(1)
                    .expect("should have at least 2 days available")
                    .date
            };
            remaining_rollout_days.push(RolloutDay {
                rollout_stages_subnets: vec![vec![self.nns_subnet]],
                date: last_day,
            })
        }

        remaining_rollout_days
            .into_iter()
            .filter(|rd| !rd.rollout_stages_subnets.is_empty())
            .collect()
    }
}

async fn get_update_states(
    network: &Network,
    prometheus_client: &prometheus_http_query::Client,
    release: &ReplicaRelease,
    since: chrono::DateTime<Utc>,
) -> Result<BTreeMap<PrincipalId, SubnetUpdateState>> {
    const STATE_FIELD: &str = "state";
    let query = format!(
        r#"
        # Get all subnets that are not yet updated to the given release. These are preparing a CUP for the update.
        label_replace(
            count by (ic_subnet) (ic_replica_info{{ic="{network}", ic_active_version!="{version}"}})
                /
            max by (ic_subnet) (consensus_dkg_current_committee_size{{ic="{network}"}})
        , 
            "{state_field}", "{preparing_state}", "", ""
        )
            or ignoring({state_field})
        # Get all subnets that are running on the given release but some nodes are not up yet. These are probably restarting to do an update.
        label_replace(
            max_over_time((count by (ic_subnet) (ic_replica_info{{ic="{network}", ic_active_version="{version}"}}))[{period}s:2m])
                <
            # max count of up replicas 10 minutes before the upgrade
            last_over_time((
                max_over_time(
                    (
                        count by (ic_subnet) (ic_replica_info{{ic_active_version!="{version}"}})
                    )[10m:1m]
                )
            )[{period}s:1m])
        ,
            "{state_field}", "{updating_state}", "", ""
        )
            or ignoring({state_field})
        # Get all subnets that have been running on the given release for at least half an hour without any restarts or pages since the specified time.
        # If the result is 1, the subnet completed the bake process successfully.
        label_replace(
            max_over_time((
                -sum_over_time(
                    (
                        sum by (ic_subnet) (ALERTS{{ic="{network}", severity="page", alertstate="firing"}})
                        -
                        # discount alerts active 10 minutes before the upgrade
                        last_over_time((
                            count by (ic_subnet) (ALERTS{{ic="{network}", severity="page", alertstate="firing"}} offset 10m)
                                and
                            count by (ic_subnet) (ic_replica_info{{ic_active_version!="{version}"}}) > 0
                        )[{period}s:1m])
                    )[30m:1m]
                ) < 0
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
        network = network.legacy_name(),
        version = release.commit_hash,
        preparing_state = SubnetUpdateState::Preparing,
        updating_state = SubnetUpdateState::Updating,
        baking_state = SubnetUpdateState::Baking,
        state_field = STATE_FIELD,
        period = Utc::now().timestamp() - since.timestamp(),
    );
    info!("release ({}) query: {}", release.commit_hash, query);
    let response = prometheus_client.query(query, None, None).await?;
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

#[derive(Serialize, Clone)]
pub struct SubnetReleaseStatus {
    pub state: SubnetUpdateState,
    pub subnet_id: PrincipalId,
    pub subnet_name: String,
    pub proposal: Option<SubnetUpdateProposal>,
    pub patches_available: Vec<ReplicaRelease>,
    pub release: ReplicaRelease,
}

pub async fn list_subnets_release_statuses(
    proposal_agent: &ProposalAgent,
    prometheus_client: &prometheus_http_query::Client,
    network: Network,
    subnets: BTreeMap<PrincipalId, Subnet>,
    releases: Vec<ReplicaRelease>,
) -> anyhow::Result<Vec<SubnetReleaseStatus>> {
    let mut subnet_update_proposals = proposal_agent.list_update_subnet_version_proposals().await?;
    subnet_update_proposals.sort_by_key(|p| p.info.proposal_timestamp_seconds);
    subnet_update_proposals.reverse();
    let latest_updates = subnets
        .keys()
        .map(|p| {
            (
                p,
                subnet_update_proposals.iter().find(|sup| sup.payload.subnet_id == *p),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let active_releases = latest_updates
        .values()
        .filter_map(|u| {
            u.map(|sup| {
                releases
                    .iter()
                    .find(|r| r.commit_hash == *sup.payload.replica_version_id)
            })
            .flatten()
        })
        .collect::<Vec<_>>();

    let oldest_release_update = active_releases
        .iter()
        .map(|r| {
            (
                r.commit_hash.clone(),
                latest_updates
                    .values()
                    .filter_map(|u| *u)
                    .filter(|u| u.payload.replica_version_id == r.commit_hash && u.info.executed)
                    .map(|u| u.info.executed_timestamp_seconds)
                    .min()
                    .map(|t| Utc.timestamp_opt(t as i64, 0).unwrap())
                    .unwrap_or_else(Utc::now),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let updates_statuses_for_revision = try_join_all(active_releases.clone().into_iter().cloned().map(|r| async {
        let retryable = || async {
            get_update_states(
                &network,
                prometheus_client,
                &r,
                *oldest_release_update.get(&r.commit_hash).expect("needs to exist"),
            )
            .await
        };
        retryable
            .retry(&ExponentialBuilder::default())
            .await
            .map(|updates| (r.commit_hash, updates))
    }))
    .await?
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    let latest_releases = releases
        .iter()
        .rev()
        .unique_by(|r| r.branch.clone())
        .collect::<Vec<_>>();

    Ok(subnets
        .values()
        .map(|s| {
            let proposal = latest_updates.get(&s.principal).expect("entry exists for each subnet");
            let release = releases
                .iter()
                .find(|r| {
                    r.commit_hash
                        == if let Some(p) = proposal {
                            p.payload.replica_version_id.clone()
                        } else {
                            s.replica_version.clone()
                        }
                })
                .expect("some release should have been found for replica");
            SubnetReleaseStatus {
                state: proposal
                    .map(|u| {
                        if !u.info.executed {
                            SubnetUpdateState::Submitted
                        } else {
                            updates_statuses_for_revision
                                .get(&u.payload.replica_version_id)
                                .expect("entry for revision must exist")
                                .get(&s.principal)
                                .cloned()
                                .unwrap_or(SubnetUpdateState::Unknown)
                        }
                    })
                    .unwrap_or(SubnetUpdateState::Unknown),
                subnet_id: s.principal,
                subnet_name: s.metadata.name.clone(),
                proposal: proposal.cloned(),
                patches_available: latest_releases
                    .iter()
                    .find(|r| r.name == release.name)
                    .map(|r| r.patches_for(&release.commit_hash).expect("patches this release"))
                    .unwrap_or_default(),
                release: release.clone(),
            }
        })
        .collect())
}

#[cfg(test)]
mod test {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_schedule() {
        let nns_subnet: PrincipalId =
            PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap();
        let canary_subnet: PrincipalId =
            PrincipalId::from_str("io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe").unwrap();
        let other_subnets: &[PrincipalId] = &[
            PrincipalId::from_str("2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe").unwrap(),
            PrincipalId::from_str("3hhby-wmtmw-umt4t-7ieyg-bbiig-xiylg-sblrt-voxgt-bqckd-a75bf-rqe").unwrap(),
            PrincipalId::from_str("4ecnw-byqwz-dtgss-ua2mh-pfvs7-c3lct-gtf4e-hnu75-j7eek-iifqm-sqe").unwrap(),
            PrincipalId::from_str("4zbus-z2bmt-ilreg-xakz4-6tyre-hsqj4-slb4g-zjwqo-snjcc-iqphi-3qe").unwrap(),
            PrincipalId::from_str("5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae").unwrap(),
            PrincipalId::from_str("6pbhf-qzpdk-kuqbr-pklfa-5ehhf-jfjps-zsj6q-57nrl-kzhpd-mu7hc-vae").unwrap(),
            PrincipalId::from_str("brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae").unwrap(),
            PrincipalId::from_str("csyj4-zmann-ys6ge-3kzi6-onexi-obayx-2fvak-zersm-euci4-6pslt-lae").unwrap(),
            PrincipalId::from_str("cv73p-6v7zi-u67oy-7jc3h-qspsz-g5lrj-4fn7k-xrax3-thek2-sl46v-jae").unwrap(),
            PrincipalId::from_str("e66qm-3cydn-nkf4i-ml4rb-4ro6o-srm5s-x5hwq-hnprz-3meqp-s7vks-5qe").unwrap(),
            PrincipalId::from_str("ejbmu-grnam-gk6ol-6irwa-htwoj-7ihfl-goimw-hlnvh-abms4-47v2e-zqe").unwrap(),
            PrincipalId::from_str("eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe").unwrap(),
            PrincipalId::from_str("fuqsr-in2lc-zbcjj-ydmcw-pzq7h-4xm2z-pto4i-dcyee-5z4rz-x63ji-nae").unwrap(),
            PrincipalId::from_str("gmq5v-hbozq-uui6y-o55wc-ihop3-562wb-3qspg-nnijg-npqp5-he3cj-3ae").unwrap(),
            PrincipalId::from_str("jtdsg-3h6gi-hs7o5-z2soi-43w3z-soyl3-ajnp3-ekni5-sw553-5kw67-nqe").unwrap(),
            PrincipalId::from_str("k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe").unwrap(),
            PrincipalId::from_str("lhg73-sax6z-2zank-6oer2-575lz-zgbxx-ptudx-5korm-fy7we-kh4hl-pqe").unwrap(),
            PrincipalId::from_str("lspz2-jx4pu-k3e7p-znm7j-q4yum-ork6e-6w4q6-pijwq-znehu-4jabe-kqe").unwrap(),
            PrincipalId::from_str("mpubz-g52jc-grhjo-5oze5-qcj74-sex34-omprz-ivnsm-qvvhr-rfzpv-vae").unwrap(),
            PrincipalId::from_str("nl6hn-ja4yw-wvmpy-3z2jx-ymc34-pisx3-3cp5z-3oj4a-qzzny-jbsv3-4qe").unwrap(),
            PrincipalId::from_str("o3ow2-2ipam-6fcjo-3j5vt-fzbge-2g7my-5fz2m-p4o2t-dwlc4-gt2q7-5ae").unwrap(),
            PrincipalId::from_str("opn46-zyspe-hhmyp-4zu6u-7sbrh-dok77-m7dch-im62f-vyimr-a3n2c-4ae").unwrap(),
            PrincipalId::from_str("pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe").unwrap(),
            PrincipalId::from_str("pjljw-kztyl-46ud4-ofrj6-nzkhm-3n4nt-wi3jt-ypmav-ijqkt-gjf66-uae").unwrap(),
            PrincipalId::from_str("qdvhd-os4o2-zzrdw-xrcv4-gljou-eztdp-bj326-e6jgr-tkhuc-ql6v2-yqe").unwrap(),
            PrincipalId::from_str("qxesv-zoxpm-vc64m-zxguk-5sj74-35vrb-tbgwg-pcird-5gr26-62oxl-cae").unwrap(),
            PrincipalId::from_str("shefu-t3kr5-t5q3w-mqmdq-jabyv-vyvtf-cyyey-3kmo4-toyln-emubw-4qe").unwrap(),
            PrincipalId::from_str("snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae").unwrap(),
            PrincipalId::from_str("uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe").unwrap(),
            PrincipalId::from_str("w4asl-4nmyj-qnr7c-6cqq4-tkwmt-o26di-iupkq-vx4kt-asbrx-jzuxh-4ae").unwrap(),
            PrincipalId::from_str("w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe").unwrap(),
            PrincipalId::from_str("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae").unwrap(),
            PrincipalId::from_str("yinp6-35cfo-wgcd2-oc4ty-2kqpf-t4dul-rfk33-fsq3r-mfmua-m2ngh-jqe").unwrap(),
        ];
        let subnets = [&[canary_subnet], other_subnets, &[nns_subnet]].concat();

        // Monday
        let week_1_monday = NaiveDate::from_ymd_opt(2022, 8, 8).unwrap();
        let week_1_tuesday = week_1_monday.succ_opt().unwrap();
        let week_1_wednesday = week_1_tuesday.succ_opt().unwrap();
        let week_1_thursday = week_1_wednesday.succ_opt().unwrap();
        let week_1_friday = week_1_thursday.succ_opt().unwrap();
        let week_1_saturday = week_1_friday.succ_opt().unwrap();
        let week_1_sunday = week_1_saturday.succ_opt().unwrap();
        let week_2_monday = week_1_sunday.succ_opt().unwrap();
        let week_2_tuesday = week_2_monday.succ_opt().unwrap();
        let week_2_wednesday = week_2_tuesday.succ_opt().unwrap();

        // Rollout not yet started
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_monday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_monday,
                rollout_stages_subnets: vec![[canary_subnet].to_vec()],
            },
            RolloutDay {
                date: week_1_tuesday,
                rollout_stages_subnets: vec![subnets[1..3].to_vec(), subnets[3..6].to_vec(), subnets[6..7].to_vec()],
            },
            RolloutDay {
                date: week_1_wednesday,
                rollout_stages_subnets: vec![
                    subnets[7..10].to_vec(),
                    subnets[10..14].to_vec(),
                    subnets[14..18].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                    subnets[26..30].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_friday,
                rollout_stages_subnets: vec![subnets[30..34].to_vec()],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "rollout started");

        // Canary submitted
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_monday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_tuesday,
                rollout_stages_subnets: vec![subnets[1..3].to_vec(), subnets[3..6].to_vec(), subnets[6..7].to_vec()],
            },
            RolloutDay {
                date: week_1_wednesday,
                rollout_stages_subnets: vec![
                    subnets[7..10].to_vec(),
                    subnets[10..14].to_vec(),
                    subnets[14..18].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                    subnets[26..30].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_friday,
                rollout_stages_subnets: vec![subnets[30..34].to_vec()],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "canary submitted");

        // First stage of the day submitted
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_tuesday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_tuesday,
                rollout_stages_subnets: vec![subnets[3..6].to_vec(), subnets[6..7].to_vec()],
            },
            RolloutDay {
                date: week_1_wednesday,
                rollout_stages_subnets: vec![
                    subnets[7..10].to_vec(),
                    subnets[10..14].to_vec(),
                    subnets[14..18].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                    subnets[26..30].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_friday,
                rollout_stages_subnets: vec![subnets[30..34].to_vec()],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "tuesday rollout under way");

        // Rollout lagging behind #1
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![
                        subnets[1..3].to_vec(),
                        subnets[3..6].to_vec(),
                        subnets[6..7].to_vec(),
                    ],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![subnets[7..10].to_vec()],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_thursday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[10..14].to_vec(),
                    subnets[14..18].to_vec(),
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_friday,
                rollout_stages_subnets: vec![subnets[26..30].to_vec(), subnets[30..34].to_vec()],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "rollout lagging behind");

        // 4 days in the week to complete the rollout
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_tuesday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_tuesday,
                rollout_stages_subnets: vec![subnets[0..1].to_vec(), subnets[1..3].to_vec()],
            },
            RolloutDay {
                date: week_1_wednesday,
                rollout_stages_subnets: vec![subnets[3..5].to_vec(), subnets[5..8].to_vec(), subnets[8..11].to_vec()],
            },
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[11..14].to_vec(),
                    subnets[14..18].to_vec(),
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_friday,
                rollout_stages_subnets: vec![subnets[26..30].to_vec(), subnets[30..34].to_vec()],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "4-day rollout");

        // 3 days in the week to complete the rollout, Friday skipped
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_tuesday,
        };

        let want = vec![
            RolloutDay {
                date: week_1_tuesday,
                rollout_stages_subnets: vec![
                    subnets[0..1].to_vec(),
                    subnets[1..3].to_vec(),
                    subnets[3..5].to_vec(),
                    subnets[5..7].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_wednesday,
                rollout_stages_subnets: vec![
                    subnets[7..9].to_vec(),
                    subnets[9..12].to_vec(),
                    subnets[12..15].to_vec(),
                    subnets[15..18].to_vec(),
                    subnets[18..21].to_vec(),
                ],
            },
            RolloutDay {
                date: week_1_thursday,
                rollout_stages_subnets: vec![
                    subnets[21..24].to_vec(),
                    subnets[24..28].to_vec(),
                    subnets[28..32].to_vec(),
                    subnets[32..34].to_vec(),
                ],
            },
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "3-day rollout");

        // Saturday after the rollout is started, but app rollout not yet complete
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_tuesday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_saturday,
        };

        let want = vec![
            RolloutDay {
                date: week_2_monday,
                rollout_stages_subnets: vec![
                    subnets[3..6].to_vec(),
                    subnets[6..10].to_vec(),
                    subnets[10..14].to_vec(),
                    subnets[14..18].to_vec(),
                    subnets[18..22].to_vec(),
                    subnets[22..26].to_vec(),
                    subnets[26..30].to_vec(),
                    subnets[30..34].to_vec(),
                ],
            },
            RolloutDay {
                date: week_2_tuesday,
                rollout_stages_subnets: vec![vec![nns_subnet]],
            },
        ];

        assert_eq!(want, state.schedule(), "rollout lagging behind on saturday");

        // Sunday after the rollout is started, and app rollout is complete
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![
                        subnets[3..6].to_vec(),
                        subnets[6..10].to_vec(),
                        subnets[10..14].to_vec(),
                        subnets[14..18].to_vec(),
                        subnets[18..22].to_vec(),
                        subnets[22..26].to_vec(),
                        subnets[26..30].to_vec(),
                        subnets[30..34].to_vec(),
                    ],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_tuesday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_sunday,
        };

        let want = vec![RolloutDay {
            date: week_2_monday,
            rollout_stages_subnets: vec![vec![nns_subnet]],
        }];

        assert_eq!(
            want,
            state.schedule(),
            "sunday, rollout should complete next day (monday)"
        );

        // Only NNS is left to deploy today
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![
                        subnets[3..6].to_vec(),
                        subnets[6..10].to_vec(),
                        subnets[10..14].to_vec(),
                        subnets[14..18].to_vec(),
                        subnets[18..22].to_vec(),
                        subnets[22..26].to_vec(),
                        subnets[26..30].to_vec(),
                        subnets[30..34].to_vec(),
                    ],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_tuesday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_2_monday,
        };

        let want = vec![RolloutDay {
            date: week_2_monday,
            rollout_stages_subnets: vec![vec![nns_subnet]],
        }];

        assert_eq!(want, state.schedule(), "monday, rollout should complete today");

        // NNS subnet update is submitted, so the rollout schedule is empty
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![
                        subnets[3..6].to_vec(),
                        subnets[6..10].to_vec(),
                        subnets[10..14].to_vec(),
                        subnets[14..18].to_vec(),
                        subnets[18..22].to_vec(),
                        subnets[22..26].to_vec(),
                        subnets[26..30].to_vec(),
                        subnets[30..34].to_vec(),
                    ],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_monday,
                    rollout_stages_subnets: vec![vec![nns_subnet]],
                },
                RolloutDay {
                    date: week_2_tuesday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_2_monday,
        };

        let want: Vec<RolloutDay> = vec![];

        assert_eq!(want, state.schedule(), "rollout complete");

        // Second week Monday is skipped
        let state = RolloutState {
            nns_subnet,
            subnets: subnets.clone(),
            rollout_days: vec![
                RolloutDay {
                    date: week_1_monday,
                    rollout_stages_subnets: vec![vec![canary_subnet]],
                },
                RolloutDay {
                    date: week_1_tuesday,
                    rollout_stages_subnets: vec![subnets[1..3].to_vec()],
                },
                RolloutDay {
                    date: week_1_wednesday,
                    rollout_stages_subnets: vec![
                        subnets[3..6].to_vec(),
                        subnets[6..10].to_vec(),
                        subnets[10..14].to_vec(),
                        subnets[14..18].to_vec(),
                        subnets[18..22].to_vec(),
                        subnets[22..26].to_vec(),
                        subnets[26..30].to_vec(),
                        subnets[30..34].to_vec(),
                    ],
                },
                RolloutDay {
                    date: week_1_thursday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_1_friday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_tuesday,
                    rollout_stages_subnets: vec![],
                },
                RolloutDay {
                    date: week_2_wednesday,
                    rollout_stages_subnets: vec![],
                },
            ],
            date: week_1_sunday,
        };

        let want = vec![RolloutDay {
            date: week_2_tuesday,
            rollout_stages_subnets: vec![vec![nns_subnet]],
        }];

        assert_eq!(
            want,
            state.schedule(),
            "monday is skipped, nns rolls out day after tomorrow"
        );
    }
}
