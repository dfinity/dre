use ic_management_types::Network;
use ic_nns_governance::pb::v1::{ListProposalInfo, ListProposalInfoResponse, ProposalInfo, ProposalStatus};

use anyhow::Result;
use candid::Decode;
use ic_agent::agent::http_transport::reqwest_transport::ReqwestTransport;
use ic_agent::Agent;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::io::Write;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};
mod slack;
use clap::Parser;
use reqwest::Url;

#[macro_use]
extern crate lazy_static;

// Time to wait for a new proposal after the last one was created before sending
// out the Slack notification.
const COOLING_PERIOD_SECS: u64 = 60;

const SLACK_URL_ENV: &str = "SLACK_URL";

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    dotenv::dotenv().ok();

    let args = Cli::parse();
    let target_network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
        .await
        .expect("Failed to create network");

    let failed_proposals_handle = tokio::spawn(notify_for_failed_proposals(target_network.clone()));
    let new_proposals_handle = tokio::spawn(notify_for_new_proposals(target_network));

    futures::future::join_all(vec![failed_proposals_handle, new_proposals_handle]).await;
}

#[derive(Parser, Debug)]
#[clap(about, version)]
struct Cli {
    // Target network. Can be one of: "mainnet", "staging", or an arbitrary "<testnet>" name
    #[clap(long, env = "NETWORK", default_value = "mainnet")]
    network: String,

    // NNS_URLs for the target network, comma separated.
    // The argument is mandatory for testnets, and is optional for mainnet and staging
    #[clap(long, env = "NNS_URLS", aliases = &["registry-url", "nns-url"], value_delimiter = ',')]
    pub nns_urls: Vec<Url>,
}

#[derive(Default)]
pub struct ProposalCheckpointStore {
    file_path: String,
    checkpoint: ProposalCheckpoint,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProposalCheckpoint {
    proposal_id: Option<u64>,
    time: Option<u64>,
}

impl ProposalCheckpointStore {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let file_path = format!("checkpoint_{name}.json");
        if std::path::Path::new(&file_path).exists() {
            let checkpoint = serde_json::from_str::<ProposalCheckpoint>(&std::fs::read_to_string(file_path)?)?;
            Ok(Self {
                file_path: format!("checkpoint_{name}.json"),
                checkpoint,
            })
        } else {
            Ok(Self {
                file_path,
                ..Default::default()
            })
        }
    }

    fn get(&self) -> ProposalCheckpoint {
        self.checkpoint.clone()
    }

    fn save(&mut self, checkpoint: ProposalCheckpoint) -> anyhow::Result<()> {
        self.checkpoint = checkpoint;
        retry::retry(retry::delay::Exponential::from_millis(10).take(5), || {
            std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&self.file_path)
                .and_then(|mut file| file.write_all(serde_json::to_string(&self.checkpoint)?.as_bytes()).map(|_| file))
                .and_then(|mut file| file.flush())
        })?;
        Ok(())
    }
}

struct ProposalPoller {
    agent: Agent,
}

impl ProposalPoller {
    fn new(target_network: Network) -> Self {
        let nns_url = target_network.get_nns_urls()[0].clone();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Could not create HTTP client.");
        let agent = Agent::builder()
            .with_transport(ReqwestTransport::create_with_client(nns_url, client).expect("failed to create transport"))
            .build()
            .expect("failed to build the agent");
        Self { agent }
    }

    pub async fn poll_pending_once(&self) -> Result<Vec<ProposalInfo>> {
        let response = self
            .agent
            .query(
                &ic_agent::export::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
                "get_pending_proposals",
            )
            .with_arg(candid::encode_one(()).expect("failed to encode arguments"))
            .call()
            .await?;

        Ok(Decode!(response.as_slice(), Vec<ProposalInfo>).expect("unable to decode proposals"))
    }

    pub async fn poll_not_executed_once(&self) -> Result<Vec<ProposalInfo>> {
        let response = self
            .agent
            .query(
                &ic_agent::export::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
                "list_proposals",
            )
            .with_arg(candid::encode_one(ListProposalInfo {
                limit: 1000,
                include_status: vec![ProposalStatus::Failed.into(), ProposalStatus::Open.into(), ProposalStatus::Adopted.into()],
                omit_large_fields: Some(true),
                ..Default::default()
            })?)
            .call()
            .await?;

        Ok(Decode!(response.as_slice(), ListProposalInfoResponse)
            .expect("unable to decode proposals")
            .proposal_info)
    }
}

async fn notify_for_new_proposals(target_network: Network) {
    let mut last_notified_proposal = ProposalCheckpointStore::new("new").expect("failed to initialize last notified proposal tracking");
    let proposal_poller = ProposalPoller::new(target_network);
    loop {
        info!("sleeping");
        sleep(Duration::from_secs(10)).await;

        info!("checking for new proposals");

        let mut proposals = proposal_poller.poll_pending_once().await.unwrap_or_default();

        proposals.sort_by(|a, b| a.id.expect("proposal has no id").id.cmp(&b.id.expect("proposal has no id").id));

        let new_proposals = proposals
            .into_iter()
            .skip_while(|proposal| {
                last_notified_proposal
                    .get()
                    .proposal_id
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

            if let Ok(message_groups) = slack::MessageGroups::try_from(new_proposals.clone()) {
                let slack_hook = slack::SlackHook::new(std::env::var(SLACK_URL_ENV).expect("SLACK_URL environment variable must be set"));

                for slack_message in message_groups.message_groups.iter() {
                    match slack_hook.send(slack_message).await {
                        Ok(response) => {
                            println!(
                                "Got a response: {}",
                                response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| { "ERROR: failed to decode the response from the slack servers".to_string() })
                            );
                        }
                        Err(e) => {
                            warn!("failed to send Slack notification: {}", e);
                            continue;
                        }
                    }
                }
                if let Err(e) = last_notified_proposal.save(ProposalCheckpoint {
                    proposal_id: last_proposal.id.expect("proposal has no id").id.into(),
                    ..Default::default()
                }) {
                    warn!("failed to save last notified proposal: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(20)).await;
    }
}

async fn notify_for_failed_proposals(target_network: Network) {
    let mut checkpoint = ProposalCheckpointStore::new("failed").expect("failed to initialize last notified proposal tracking");
    let proposal_poller = ProposalPoller::new(target_network);
    loop {
        info!("checking for failed proposals");
        if let Ok(mut proposals) = proposal_poller.poll_not_executed_once().await {
            proposals.sort_by(|a, b| a.id.expect("proposal has no id").id.cmp(&b.id.expect("proposal has no id").id));

            let pending_proposals = proposals
                .into_iter()
                .skip_while(|proposal| proposal.id.expect("proposal has no id").id < checkpoint.get().proposal_id.unwrap_or_default())
                .collect::<Vec<_>>();
            let oldest_pending_proposal: Option<ProposalInfo> = pending_proposals.first().cloned();
            let mut new_failed_proposals = pending_proposals
                .into_iter()
                .filter(|proposal| {
                    ProposalStatus::try_from(proposal.status).expect("invalid proposal status") == ProposalStatus::Failed
                        && proposal.failed_timestamp_seconds > checkpoint.get().time.unwrap_or_default()
                })
                .collect::<Vec<_>>();

            new_failed_proposals.sort_by(|a, b| a.failed_timestamp_seconds.cmp(&b.failed_timestamp_seconds));

            if !new_failed_proposals.is_empty() {
                info!("new proposals: {:?}", &new_failed_proposals);
            }

            if let Ok(message_groups) = slack::MessageGroups::try_from(new_failed_proposals.clone()) {
                let slack_hook = slack::SlackHook::new(std::env::var(SLACK_URL_ENV).expect("SLACK_URL environment variable must be set"));

                for slack_message in message_groups.message_groups.iter() {
                    match slack_hook.send(slack_message).await {
                        Ok(response) => {
                            println!(
                                "Got a response: {}",
                                response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| { "ERROR: failed to decode the response from the slack servers".to_string() })
                            );
                        }
                        Err(e) => {
                            warn!("failed to send Slack notification: {}", e);
                            continue;
                        }
                    }
                }

                if let Err(e) = checkpoint.save(ProposalCheckpoint {
                    proposal_id: oldest_pending_proposal.map(|p| p.id.expect("proposal has no id").id),
                    time: new_failed_proposals.last().map(|p| p.failed_timestamp_seconds),
                }) {
                    warn!("failed to save last notified proposal: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(20)).await;
    }
}
