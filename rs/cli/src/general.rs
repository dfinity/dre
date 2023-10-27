use std::time::Duration;

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

    loop {
        info!("Refreshing proposals...");
        let proposals = client.get_pending_proposals().await?;
        let proposals: Vec<&ProposalInfo> = proposals
            .iter()
            .filter(|p| accepted_topics.contains(&p.topic) || !accepted_proposers.contains(&p.proposer.unwrap().id))
            .collect();
        info!("Found total of {} proposals", proposals.len());

        for proposal in proposals {
            info!(
                "Voting on proposal {} -> {}",
                proposal.id.unwrap().id,
                proposal.proposal.clone().unwrap().title.unwrap()
            );

            client.register_vote(neuron.id, proposal.id.unwrap().id).await?;
            info!("Successfully voted on proposal {}", proposal.id.unwrap().id);
        }

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
