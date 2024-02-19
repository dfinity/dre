use candid::Decode;
use dialoguer::Confirm;
use ic_base_types::{CanisterId, PrincipalId};
use ic_management_backend::registry::RegistryState;
use ic_management_types::Network;
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
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
use ic_nns_governance::pb::v1::{ExecuteNnsFunction, ProposalInfo};
use log::{error, info, warn};
use url::Url;

use crate::detect_neuron::{Auth, Neuron};

pub(crate) async fn vote_on_proposals(
    neuron: &Neuron,
    nns_url: &Url,
    accepted_proposers: &[u64],
    accepted_topics: &[i32],
    simulate: bool,
) -> anyhow::Result<()> {
    let client: GovernanceCanisterWrapper = match &neuron.auth {
        Auth::Hsm { pin, slot, key_id } => {
            CanisterClient::from_hsm(pin.to_string(), *slot, key_id.to_string(), nns_url)?.into()
        }
        Auth::Keyfile { path } => CanisterClient::from_key_file(path.into(), nns_url)?.into(),
    };

    // In case of incorrectly set voting following, or in case of some other errors,
    // we don't want to vote on the same proposal multiple times. So we keep an
    // in-memory set of proposals that we already voted on.
    let mut voted_proposals = HashSet::new();

    loop {
        // We create a new instance each time in order to update the local registry in
        // case there was a new version voted in during the execution of a script
        let registry_state = RegistryState::new(Network::Url(nns_url.clone()), false).await;
        let latest_version = match registry_state.get_blessed_replica_versions().await {
            Ok(v) => {
                let binding = v.first();
                match binding {
                    Some(str) => str.to_string(),
                    None => "".to_string(),
                }
            }
            Err(e) => {
                error!("Couldn't get last replica versions: {:?}", e);
                "".to_string()
            }
        };

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
            let content = proposal.proposal.as_ref().unwrap().action.as_ref().unwrap();
            let parsed = match content {
                ic_nns_governance::pb::v1::proposal::Action::ExecuteNnsFunction(ExecuteNnsFunction {
                    nns_function: _,
                    payload,
                }) => match Decode!(&payload, UpdateSubnetReplicaVersionPayload) {
                    Ok(payload) => payload,
                    Err(e) => {
                        error!("Couldn't decode into update subnet replica version payload: {:?}", e);
                        continue;
                    }
                },
                _ => {
                    warn!("Proposal's content wasn't correct. Skipping...");
                    continue;
                }
            };

            if parsed.replica_version_id != *latest_version
                && !Confirm::new()
                    .with_prompt(format!("CAUTION! You are about to vote on a proposal that will update the subnet '{}' to a non-latest version! Do you want to continue?", parsed.subnet_id.0.to_string().split('-').collect::<Vec<_>>().first().unwrap()))
                    .default(false)
                    .interact()?
            {
                info!("Skipping voting on proposal {}", proposal.id.as_ref().unwrap().id);
                voted_proposals.insert(proposal.id.unwrap().id);
                continue;
            }

            info!(
                "Voting on proposal {} (topic {:?}, proposer {}) -> {}",
                proposal.id.unwrap().id,
                proposal.topic(),
                proposal.proposer.unwrap_or_default().id,
                proposal.proposal.as_ref().unwrap().title.as_ref().unwrap()
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

pub(crate) async fn get_node_metrics_history(
    wallet: CanisterId,
    subnets: Vec<PrincipalId>,
    start_at_nanos: u64,
    neuron: &Neuron,
    nns_url: &Url,
) -> anyhow::Result<()> {
    let lock = Mutex::new(());
    let canister_agent = match &neuron.auth {
        Auth::Hsm { pin, slot, key_id } => {
            IcAgentCanisterClient::from_hsm(pin.to_string(), *slot, key_id.to_string(), nns_url.clone(), Some(lock))?
        }
        Auth::Keyfile { path } => IcAgentCanisterClient::from_key_file(path.into(), nns_url.clone())?,
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
