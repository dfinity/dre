use spinners::{Spinner, Spinners};
use std::{collections::HashSet, io::Write, time::Duration};

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
    simulate: bool,
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
