use ic_nns_governance::pb::v1::ProposalInfo;

use anyhow::Result;
use candid::Decode;
use ic_agent::Agent;
use log::{info, warn};
use serde::Deserialize;
use std::convert::TryFrom;
use std::io::Write;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};
mod slack;

#[macro_use]
extern crate lazy_static;

#[derive(Deserialize)]
struct Config {}

// Time to wait for a new proposal after the last one was created before sending out the Slack notification.
const COOLING_PERIOD_SECS: u64 = 60;

const SLACK_URL_ENV: &str = "SLACK_URL";
const SLACK_CHANNEL_ENV: &str = "SLACK_CHANNEL";

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    dotenv::dotenv().ok();

    let proposal_poller = ProposalPoller::new();

    let mut slack_hook = slack::SlackHook::new(std::env::var(SLACK_URL_ENV).expect("SLACK_URL must be set"));
    if let Ok(channel) = std::env::var(SLACK_CHANNEL_ENV) {
        slack_hook = slack_hook.channel(channel);
    }

    let mut last_notified_proposal =
        LastNotifiedProposal::new().expect("failed to initialize last notified proposal tracking");

    loop {
        info!("checking for new proposals");

        let mut proposals = proposal_poller.poll_once().await.unwrap_or_default();

        proposals.sort_by(|a, b| {
            a.id.expect("proposal has no id")
                .id
                .cmp(&b.id.expect("proposal has no id").id)
        });

        let new_proposals = proposals
            .into_iter()
            .skip_while(|proposal| {
                last_notified_proposal
                    .get()
                    .map(|last_notified| proposal.id.expect("proposal has no id").id <= last_notified)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        if !new_proposals.is_empty() {
            info!("new proposals: {:?}", &new_proposals);
        }

        if let Some(last_proposal) = new_proposals.last() {
            let secs_since_last_proposal = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time incorrect")
                .as_secs()
                - last_proposal.proposal_timestamp_seconds;
            if secs_since_last_proposal < COOLING_PERIOD_SECS {
                sleep(Duration::from_secs(COOLING_PERIOD_SECS - secs_since_last_proposal + 1)).await;
                continue;
            }

            if let Ok(payload) = slack::SlackPayload::try_from(new_proposals.clone()) {
                match slack_hook.send(&payload).await {
                    Ok(_) => {
                        if let Err(e) = last_notified_proposal.save(last_proposal.id.expect("proposal has no id").id) {
                            warn!("failed to save last notified proposal: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("failed to send Slack notification: {}", e);
                    }
                }
            }
        }

        sleep(Duration::from_secs(20)).await;
    }
}

pub struct LastNotifiedProposal {
    file_path: String,
    last_notified_proposal_id: Option<u64>,
}

impl LastNotifiedProposal {
    pub fn new() -> anyhow::Result<Self> {
        let default_file_path = "last_notified_proposal_id".to_string();

        if std::path::Path::new(&default_file_path).exists() {
            Ok(Self {
                file_path: default_file_path.clone(),
                last_notified_proposal_id: std::fs::read_to_string(default_file_path)?.parse::<u64>()?.into(),
            })
        } else {
            Ok(Self {
                last_notified_proposal_id: None,
                file_path: default_file_path,
            })
        }
    }

    fn get(&self) -> Option<u64> {
        self.last_notified_proposal_id
    }

    fn save(&mut self, id: u64) -> anyhow::Result<()> {
        retry::retry(retry::delay::Exponential::from_millis(10).take(5), || {
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&self.file_path)
                .and_then(|mut file| file.write_all(id.to_string().as_bytes()).map(|_| file))
                .and_then(|mut file| file.flush())
        })?;
        self.last_notified_proposal_id = Some(id);
        Ok(())
    }
}

struct ProposalPoller {
    agent: Agent,
}

impl ProposalPoller {
    fn new() -> Self {
        let agent = Agent::builder()
            .with_transport(
                ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport::create("https://nns.ic0.app")
                    .expect("failed to create transport"),
            )
            .build()
            .expect("failed to build the agent");
        Self { agent }
    }

    pub async fn poll_once(&self) -> Result<Vec<ProposalInfo>> {
        let response = self
            .agent
            .query(
                &ic_types::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
                "get_pending_proposals",
            )
            .with_arg(candid::IDLArgs::new(&[]).to_bytes().unwrap().as_slice())
            .call()
            .await?;

        Ok(Decode!(response.as_slice(), Vec<ProposalInfo>).expect("unable to decode proposals"))
    }
}
