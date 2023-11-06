use chrono::{DateTime, Utc};
use std::{collections::HashSet, time::Duration};

use ic_canisters::GovernanceCanisterWrapper;
use ic_nns_governance::pb::v1::ProposalInfo;
use log::info;
use url::Url;

use crate::detect_neuron::{Auth, Neuron};

pub(crate) async fn vote_on_proposals(
    neuron: &Neuron,
    nns_url: &Url,
    accepted_proposers: &[u64],
    accepted_topics: &[i32],
) -> anyhow::Result<()> {
    let client = match &neuron.auth {
        Auth::Hsm { pin, slot, key_id } => {
            GovernanceCanisterWrapper::from_hsm(pin.to_string(), *slot, key_id.to_string(), nns_url)?
        }
        Auth::Keyfile { path } => GovernanceCanisterWrapper::from_key_file(path.into(), nns_url)?,
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
        info!(
            "Pending proposals {}, will vote on {} of them",
            proposals.len(),
            proposals_to_vote.len()
        );

        for proposal in proposals_to_vote.into_iter() {
            info!(
                "Voting on proposal {} (topic {:?}, proposer {}) -> {}",
                proposal.id.unwrap().id,
                proposal.topic(),
                proposal.proposer.unwrap_or_default().id,
                proposal.proposal.clone().unwrap().title.unwrap()
            );

            let response = client.register_vote(neuron.id, proposal.id.unwrap().id).await?;
            info!("{}", response);
            voted_proposals.insert(proposal.id.unwrap().id);
        }

        let current_utc: DateTime<Utc> = Utc::now();
        info!(
            "{} UTC: sleeping 15s before another check for pending proposals...",
            current_utc.format("%Y-%m-%d %H:%M:%S")
        );
        let sleep = tokio::time::sleep(Duration::from_secs(15));
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, exiting...");
                break;
            }
            _ = sleep => continue
        }
    }

    Ok(())
}
