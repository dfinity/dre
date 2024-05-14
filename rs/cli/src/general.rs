use ic_base_types::{CanisterId, PrincipalId};
use ic_management_types::Network;
use ic_nns_common::pb::v1::ProposalId;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use spinners::{Spinner, Spinners};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::Mutex,
    time::Duration,
};

use ic_canisters::{
    governance::GovernanceCanisterWrapper, management::WalletCanisterWrapper, registry::RegistryCanisterWrapper,
    CanisterClient, IcAgentCanisterClient,
};
use ic_nns_governance::pb::v1::{ListProposalInfo, ProposalInfo, ProposalStatus, Topic};
use log::{info, warn};
use url::Url;

use crate::detect_neuron::{Auth, Neuron};

pub async fn vote_on_proposals(
    neuron: &Neuron,
    nns_urls: &[Url],
    accepted_proposers: &[u64],
    accepted_topics: &[i32],
    simulate: bool,
) -> anyhow::Result<()> {
    let client: GovernanceCanisterWrapper = match &neuron.auth {
        Auth::Hsm { pin, slot, key_id } => {
            CanisterClient::from_hsm(pin.to_string(), *slot, key_id.to_string(), &nns_urls[0])?.into()
        }
        Auth::Keyfile { path } => CanisterClient::from_key_file(path.into(), &nns_urls[0])?.into(),
    };

    // In case of incorrectly set voting following, or in case of some other errors,
    // we don't want to vote on the same proposal multiple times. So we keep an
    // in-memory set of proposals that we already voted on.
    let mut voted_proposals = HashSet::new();

    loop {
        let proposals = client.get_pending_proposals().await?;
        let proposals: Vec<&ProposalInfo> = proposals
            .iter()
            .filter(|p| accepted_topics.contains(&p.topic) && accepted_proposers.contains(&p.proposer.unwrap().id))
            .collect();
        let proposals_to_vote = proposals
            .iter()
            .filter(|p| !voted_proposals.contains(&p.id.unwrap().id))
            .collect::<Vec<_>>();

        // Clear last line in terminal
        print!("\x1B[1A\x1B[K");
        std::io::stdout().flush().unwrap();
        for proposal in proposals_to_vote.into_iter() {
            info!(
                "Voting on proposal {} (topic {:?}, proposer {}) -> {}",
                proposal.id.unwrap().id,
                proposal.topic(),
                proposal.proposer.unwrap_or_default().id,
                proposal.proposal.clone().unwrap().title.unwrap()
            );

            if !simulate {
                let response = client.register_vote(neuron.id, proposal.id.unwrap().id).await?;
                info!("{}", response);
            } else {
                info!("Simulating vote");
            }
            voted_proposals.insert(proposal.id.unwrap().id);
        }

        let mut sp = Spinner::with_timer(
            Spinners::Dots12,
            "Sleeping 15s before another check for pending proposals...".into(),
        );
        let sleep = tokio::time::sleep(Duration::from_secs(15));
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, exiting...");
                sp.stop();
                break;
            }
            _ = sleep => {
                sp.stop_with_message("Done sleeping, checking for pending proposals...".into());
                continue
            }
        }
    }

    Ok(())
}

pub async fn get_node_metrics_history(
    wallet: CanisterId,
    subnets: Vec<PrincipalId>,
    start_at_nanos: u64,
    auth: &Auth,
    nns_urls: &[Url],
) -> anyhow::Result<()> {
    let lock = Mutex::new(());
    let canister_agent = match auth {
        Auth::Hsm { pin, slot, key_id } => IcAgentCanisterClient::from_hsm(
            pin.to_string(),
            *slot,
            key_id.to_string(),
            nns_urls[0].clone(),
            Some(lock),
        )?,
        Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.into(), nns_urls[0].clone())?,
    };
    info!("Started action...");
    let wallet_client = WalletCanisterWrapper::new(canister_agent.agent.clone());

    let subnets = match subnets.is_empty() {
        false => subnets,
        true => {
            let registry_client = RegistryCanisterWrapper::new(canister_agent.agent);
            registry_client.get_subnets().await?
        }
    };

    let mut metrics_by_subnet = HashMap::new();
    info!("Running in parallel mode");
    let mut handles = vec![];
    for subnet in subnets {
        info!("Spawning thread for subnet: {}", subnet);
        let current_client = wallet_client.clone();
        handles.push(tokio::spawn(async move {
            (
                subnet,
                current_client
                    .get_node_metrics_history(wallet, start_at_nanos, subnet)
                    .await,
            )
        }))
    }
    for handle in handles {
        let (subnet, resp) = handle.await?;
        match resp {
            Ok(metrics) => {
                info!("Received response for subnet: {}", subnet);
                metrics_by_subnet.insert(subnet, metrics);
            }
            Err(e) => {
                warn!("Couldn't fetch trustworthy metrics for subnet {}: {:?}", subnet, e)
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&metrics_by_subnet)?);

    Ok(())
}

pub async fn filter_proposals(
    network: Network,
    limit: &u32,
    statuses: Vec<ProposalStatus>,
    topics: Vec<Topic>,
) -> anyhow::Result<()> {
    let nns_url = match network.get_nns_urls().first() {
        Some(url) => url,
        None => return Err(anyhow::anyhow!("Could not get NNS URL from network config")),
    };
    let client = GovernanceCanisterWrapper::from(CanisterClient::from_anonymous(nns_url)?);

    let mut remaining = *limit;
    let mut proposals: Vec<Proposal> = vec![];
    let mut payload = ListProposalInfo { ..Default::default() };
    info!(
        "Querying {} proposals where status is one of {:?} and topic is one of {:?}",
        limit, statuses, topics
    );
    loop {
        let current_batch = client
            .list_proposals(payload)
            .await?
            .into_iter()
            .map(|f| f.into())
            .sorted_by(|a: &Proposal, b: &Proposal| b.id.cmp(&a.id))
            .collect_vec();
        payload = ListProposalInfo {
            before_proposal: current_batch.clone().last().map(|p| ProposalId { id: p.id }),
            ..Default::default()
        };
        let current_batch = current_batch
            .into_iter()
            .filter(|p| {
                if !statuses.is_empty() && !statuses.contains(&p.status) {
                    return false;
                }
                if !topics.is_empty() && !topics.contains(&p.topic) {
                    return false;
                }
                true
            })
            .collect_vec();

        if current_batch.len() > remaining as usize {
            let current_batch = current_batch.into_iter().take(remaining as usize).collect_vec();
            remaining = 0;
            proposals.extend(current_batch)
        } else {
            remaining -= current_batch.len() as u32;
            proposals.extend(current_batch)
        }

        info!("Remaining after iteration: {}", remaining);

        if remaining == 0 {
            break;
        }

        if let None = payload.before_proposal {
            warn!(
                "No more proposals available and there is {} remaining to find",
                remaining
            );
            break;
        }
    }
    println!("{}", serde_json::to_string_pretty(&proposals)?);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Proposal {
    id: u64,
    proposer: u64,
    title: String,
    summary: String,
    proposal_timestamp_seconds: u64,
    topic: Topic,
    status: ProposalStatus,
}

impl From<ProposalInfo> for Proposal {
    fn from(value: ProposalInfo) -> Self {
        let proposal = value.proposal.clone().unwrap();
        Self {
            id: value.id.unwrap().id,
            proposal_timestamp_seconds: value.proposal_timestamp_seconds,
            proposer: value.proposer.unwrap().id,
            status: value.status(),
            summary: proposal.summary,
            title: proposal.title.unwrap_or_default(),
            topic: value.topic(),
        }
    }
}
