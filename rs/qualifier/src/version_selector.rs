use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use rollouts::{RolloutState, Rollouts};

pub struct StartVersionSelectorBuilder {
    client_builder: ClientBuilder,
    rollout_dashboard_url: Option<String>,
}

#[allow(dead_code)]
impl StartVersionSelectorBuilder {
    pub fn new() -> Self {
        Self {
            client_builder: ClientBuilder::new(),
            rollout_dashboard_url: None,
        }
    }

    pub fn with_client(self, client_builder: ClientBuilder) -> Self {
        Self { client_builder, ..self }
    }

    pub fn with_rollout_dashboard_url(self, rollout_dashboard_url: &str) -> Self {
        Self {
            rollout_dashboard_url: Some(rollout_dashboard_url.to_string()),
            ..self
        }
    }

    pub async fn build(self) -> anyhow::Result<StartVersionSelector> {
        let client = self.client_builder.build()?;

        StartVersionSelector::new(client, self.rollout_dashboard_url.unwrap_or(DASHBOARD_URL.to_string())).await
    }
}

const DASHBOARD_URL: &str = "https://rollout-dashboard.ch1-rel1.dfinity.network/api/v1/rollouts";
const NNS_SUBNET_ID: &str = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe";
pub struct StartVersionSelector {
    rollouts: Rollouts,
}

impl StartVersionSelector {
    async fn new(client: Client, rollout_dashboard_url: String) -> anyhow::Result<Self> {
        let rollouts_retry = || async { client.get(&rollout_dashboard_url).send().await };
        let response = rollouts_retry
            .retry(&ExponentialBuilder::default().with_max_delay(Duration::from_secs(60)).with_max_times(5))
            .await?;

        let rollouts = response.error_for_status()?.json::<Rollouts>().await?;

        Ok(Self { rollouts })
    }

    pub fn get_forcasted_version_for_mainnet_nns(&self) -> anyhow::Result<String> {
        let rollout = self
            .rollouts
            .iter()
            .filter(|r| r.state > RolloutState::Failed && r.state < RolloutState::Complete)
            // with this we basically reverse the sorting
            // because iterator doesn't have a func that returns
            // first, but only has the one returning last.
            // This avoids one unneeded materialization
            .sorted_by_key(|r| -r.dispatch_time.timestamp())
            .last()
            .ok_or(anyhow::anyhow!(
                "No active rollouts found in the API. All rollouts: \n{:#?}",
                self.rollouts
            ))?;
        rollout
            .batches
            .iter()
            .find_map(|(_, b)| b.subnets.iter().find(|s| s.subnet_id.eq(NNS_SUBNET_ID)).cloned().map(|s| s.git_revision))
            .ok_or(anyhow::anyhow!("Couldn't find NNS in the active rollout: \n{:#?}", rollout))
    }
}

// TODO: replace with dre-airflow once its public
#[allow(dead_code)]
mod rollouts {
    use chrono::{DateTime, Utc};
    use indexmap::IndexMap;
    use serde::Deserialize;
    use serde::Serialize;
    use std::collections::HashMap;
    use std::vec::Vec;
    use strum::Display;

    #[derive(Serialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Display, Deserialize)]
    #[serde(rename_all = "snake_case")]
    /// Represents the rollout state of a subnet.
    // Ordering matters here.
    pub enum SubnetRolloutState {
        Error,
        PredecessorFailed,
        Pending,
        Waiting,
        Proposing,
        WaitingForElection,
        WaitingForAdoption,
        WaitingForAlertsGone,
        Complete,
        Unknown,
    }

    #[derive(Serialize, Debug, Clone, Deserialize)]
    /// Represents a subnet to be upgraded as part of a batch in a rollout.
    pub struct Subnet {
        /// Long-form subnet ID.
        pub subnet_id: String,
        /// Git revision of the IC OS GuestOS to deploy to the subnet.
        pub git_revision: String,
        pub state: SubnetRolloutState,
        /// Shows a comment for the subnet if it is available; else it contains an empty string.
        pub comment: String,
        /// Links to the specific task within Airflow that this subnet is currently performing; else it contains an empty string.
        pub display_url: String,
    }

    #[derive(Serialize, Debug, Clone, Deserialize)]
    /// Represents a batch of subnets to upgrade.
    pub struct Batch {
        /// The time the batch was programmed to start at.
        pub planned_start_time: DateTime<Utc>,
        /// The actual observed start time of the batch.
        pub actual_start_time: Option<DateTime<Utc>>,
        /// The time of the last action associated with this batch, if any.
        pub end_time: Option<DateTime<Utc>>,
        /// A list of subnets to be upgraded as part of this batch.
        pub subnets: Vec<Subnet>,
    }
    impl Batch {}

    #[derive(Serialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Deserialize)]
    #[serde(rename_all = "snake_case")]
    /// Represents the rollout state.
    // Ordering matters here.
    pub enum RolloutState {
        /// The rollout has failed or was abandoned by the operator.  It is not executing any longer.
        Failed,
        /// The rollout is experiencing a retryable issue.  It continues to execute.
        Problem,
        /// The rollout is in the planning stage.
        Preparing,
        /// The rollout is waiting until all preconditions have been met.
        Waiting,
        /// The rollout is upgrading subnets batch by batch.
        UpgradingSubnets,
        /// The rollout is upgrading unassigned nodes.
        UpgradingUnassignedNodes,
        /// The rollout has finished successfully or was marked as such by the operator.
        Complete,
    }

    #[derive(Debug, Serialize, Clone, Deserialize)]
    /// Represents an IC OS rollout.
    pub struct Rollout {
        /// Unique, enforced by Airflow, corresponds to DAG run ID.
        pub name: String,
        /// Link to the rollout screen in Airflow.
        pub display_url: String,
        /// Note set on the rollout by the operator.
        pub note: Option<String>,
        pub state: RolloutState,
        pub dispatch_time: DateTime<Utc>,
        /// Last scheduling decision.
        /// Due to the way the central rollout cache is updated, clients may not see
        /// an up-to-date value that corresponds to Airflow's last update time for
        /// the DAG run.  See documentation in function `get_rollout_data`.
        pub last_scheduling_decision: Option<DateTime<Utc>>,
        /// Associative array of `{batch ID -> Batch}` planned for the rollout.
        pub batches: IndexMap<usize, Batch>,
        /// Configuration associated to the rollout.
        pub conf: HashMap<String, serde_json::Value>,
    }

    impl Rollout {
        pub fn new(
            name: String,
            display_url: String,
            note: Option<String>,
            dispatch_time: DateTime<Utc>,
            last_scheduling_decision: Option<DateTime<Utc>>,
            conf: HashMap<String, serde_json::Value>,
        ) -> Self {
            Self {
                name,
                display_url,
                note,
                state: RolloutState::Complete,
                dispatch_time,
                last_scheduling_decision,
                batches: IndexMap::new(),
                conf,
            }
        }
    }

    /// List of rollouts.
    ///
    /// The API call `/api/v1/rollouts` returns this in JSON format as its content,
    /// when the information the rollout dashboard backend has collected is
    /// complete and free of errors.
    ///
    /// Rollouts are always returned in reverse chronological order -- the most
    /// recent comes first, and the last item is the oldest rollout.
    pub type Rollouts = Vec<Rollout>;
}
