use candid::Decode;
use cycles_minting_canister::SetAuthorizedSubnetworkListArgs;
use ic_base_types::{CanisterId, PrincipalId};
use ic_management_types::Network;
use ic_nervous_system_clients::canister_id_record::CanisterIdRecord;
use ic_nervous_system_root::change_canister::{AddCanisterRequest, ChangeCanisterRequest, StopOrStartCanisterRequest};
use ic_nns_common::{pb::v1::ProposalId, types::UpdateIcpXdrConversionRatePayload};
use ic_protobuf::registry::{
    dc::v1::AddOrRemoveDataCentersProposalPayload, node_operator::v1::RemoveNodeOperatorsPayload,
    node_rewards::v2::UpdateNodeRewardsTableProposalPayload,
};
use ic_sns_wasm::pb::v1::{
    AddWasmRequest, InsertUpgradePathEntriesRequest, UpdateAllowedPrincipalsRequest, UpdateSnsSubnetListRequest,
};
use itertools::Itertools;
use registry_canister::mutations::{
    complete_canister_migration::CompleteCanisterMigrationPayload,
    do_add_api_boundary_nodes::AddApiBoundaryNodesPayload,
    do_add_node_operator::AddNodeOperatorPayload,
    do_add_nodes_to_subnet::AddNodesToSubnetPayload,
    do_bless_replica_version::BlessReplicaVersionPayload,
    do_change_subnet_membership::ChangeSubnetMembershipPayload,
    do_create_subnet::CreateSubnetPayload,
    do_deploy_guestos_to_all_subnet_nodes::DeployGuestosToAllSubnetNodesPayload,
    do_deploy_guestos_to_all_unassigned_nodes::DeployGuestosToAllUnassignedNodesPayload,
    do_recover_subnet::RecoverSubnetPayload,
    do_remove_api_boundary_nodes::RemoveApiBoundaryNodesPayload,
    do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload,
    do_retire_replica_version::RetireReplicaVersionPayload,
    do_revise_elected_replica_versions::ReviseElectedGuestosVersionsPayload,
    do_set_firewall_config::SetFirewallConfigPayload,
    do_update_api_boundary_nodes_version::UpdateApiBoundaryNodesVersionPayload,
    do_update_elected_hostos_versions::UpdateElectedHostosVersionsPayload,
    do_update_node_operator_config::UpdateNodeOperatorConfigPayload,
    do_update_nodes_hostos_version::UpdateNodesHostosVersionPayload,
    do_update_ssh_readonly_access_for_all_unassigned_nodes::UpdateSshReadOnlyAccessForAllUnassignedNodesPayload,
    do_update_subnet::UpdateSubnetPayload,
    do_update_unassigned_nodes_config::UpdateUnassignedNodesConfigPayload,
    firewall::{AddFirewallRulesPayload, RemoveFirewallRulesPayload, UpdateFirewallRulesPayload},
    node_management::do_remove_nodes::RemoveNodesPayload,
    prepare_canister_migration::PrepareCanisterMigrationPayload,
    reroute_canister_ranges::RerouteCanisterRangesPayload,
};
use serde::{Deserialize, Serialize};
use spinners::{Spinner, Spinners};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::Mutex,
    time::Duration,
};
use strum::IntoEnumIterator;

use ic_canisters::{
    governance::GovernanceCanisterWrapper, management::WalletCanisterWrapper, registry::RegistryCanisterWrapper,
    CanisterClient, IcAgentCanisterClient,
};
use ic_nns_governance::{
    governance::{BitcoinSetConfigProposal, SubnetRentalRequest},
    pb::v1::{proposal::Action, ListProposalInfo, ProposalInfo, ProposalStatus, Topic},
};
use log::{error, info, warn};
use url::Url;

use crate::detect_neuron::{Auth, Neuron};

pub async fn vote_on_proposals(
    neuron: &Neuron,
    nns_urls: &[Url],
    accepted_proposers: &[u64],
    accepted_topics: &[i32],
    simulate: bool,
) -> anyhow::Result<()> {
    let client: GovernanceCanisterWrapper = match &neuron.get_auth(false).await? {
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
                let response = client
                    .register_vote(neuron.get_neuron_id(false).await?, proposal.id.unwrap().id)
                    .await?;
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

    let exclude_topic = match topics.is_empty() {
        true => vec![],
        false => {
            let mut all_topics = Topic::iter().collect_vec();
            for topic in &topics {
                all_topics.retain(|t| t != topic);
            }
            all_topics
        }
    };

    let mut remaining = *limit;
    let mut proposals: Vec<Proposal> = vec![];
    let mut payload = ListProposalInfo {
        before_proposal: None,
        exclude_topic: exclude_topic.clone().into_iter().map(|t| t.into()).collect_vec(),
        include_status: statuses.clone().into_iter().map(|s| s.into()).collect_vec(),
        include_all_manage_neuron_proposals: Some(true),
        ..Default::default()
    };
    info!(
        "Querying {} proposals where status is {} and topic is {}",
        limit,
        match statuses.is_empty() {
            true => "any".to_string(),
            false => format!("{:?}", statuses),
        },
        match exclude_topic.is_empty() {
            true => "any".to_string(),
            false => format!("not in {:?}", exclude_topic),
        }
    );
    loop {
        let current_batch = client
            .list_proposals(payload)
            .await?
            .into_iter()
            .filter_map(|p| match p.clone().try_into() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Error converting proposal info {:?}: {:?}", p, e);
                    None
                }
            })
            .sorted_by(|a: &Proposal, b: &Proposal| b.id.cmp(&a.id))
            .collect_vec();
        payload = ListProposalInfo {
            before_proposal: current_batch.clone().last().map(|p| ProposalId { id: p.id }),
            exclude_topic: exclude_topic.clone().into_iter().map(|t| t.into()).collect_vec(),
            include_status: statuses.clone().into_iter().map(|s| s.into()).collect_vec(),
            include_all_manage_neuron_proposals: Some(true),
            ..Default::default()
        };

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

        if payload.before_proposal.is_none() {
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
    payload: String,
}

impl TryFrom<ProposalInfo> for Proposal {
    fn try_from(value: ProposalInfo) -> Result<Self, Self::Error> {
        let proposal = value.proposal.clone().unwrap();
        Ok(Self {
            id: value.id.unwrap().id,
            proposal_timestamp_seconds: value.proposal_timestamp_seconds,
            proposer: value.proposer.unwrap().id,
            status: value.status(),
            summary: proposal.summary,
            title: proposal.title.unwrap_or_default(),
            topic: value.topic(),
            payload: match proposal.action.unwrap() {
                Action::ManageNeuron(a) => serde_json::to_string(&a.command)?,
                Action::ManageNetworkEconomics(a) => serde_json::to_string(&a)?,
                Action::Motion(a) => serde_json::to_string(&a)?,
                Action::ExecuteNnsFunction(a) => {
                    if a.payload.is_empty() {
                        "".to_string()
                    } else {
                        match a.nns_function() {
                            ic_nns_governance::pb::v1::NnsFunction::Unspecified => serde_json::to_string(&a)?,
                            ic_nns_governance::pb::v1::NnsFunction::CreateSubnet => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), CreateSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddNodeToSubnet => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddNodesToSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::NnsCanisterInstall => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddCanisterRequest)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::NnsCanisterUpgrade => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), ChangeCanisterRequest))?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::BlessReplicaVersion => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), BlessReplicaVersionPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RecoverSubnet => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RecoverSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateConfigOfSubnet => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AssignNoid => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddNodeOperatorPayload)?))?
                            }
                            // Unable to resolve rustls deps when adding `ic-nns-test-utils`
                            ic_nns_governance::pb::v1::NnsFunction::NnsRootUpgrade => "".to_string(),
                            ic_nns_governance::pb::v1::NnsFunction::IcpXdrConversionRate => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateIcpXdrConversionRatePayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToAllSubnetNodes => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), DeployGuestosToAllSubnetNodesPayload)?),
                                )?
                            }
                            // Has an empty payload
                            ic_nns_governance::pb::v1::NnsFunction::ClearProvisionalWhitelist => "".to_string(),
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodesFromSubnet => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RemoveNodesFromSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::SetAuthorizedSubnetworks => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), SetAuthorizedSubnetworkListArgs)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::SetFirewallConfig => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), SetFirewallConfigPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodeOperatorConfig => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateNodeOperatorConfigPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::StopOrStartNnsCanister => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), StopOrStartCanisterRequest)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodes => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RemoveNodesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UninstallCode => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), CanisterIdRecord)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodeRewardsTable => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateNodeRewardsTableProposalPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::AddOrRemoveDataCenters => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), AddOrRemoveDataCentersProposalPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::UpdateUnassignedNodesConfig => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), UpdateUnassignedNodesConfigPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodeOperators => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RemoveNodeOperatorsPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RerouteCanisterRanges => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RerouteCanisterRangesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddFirewallRules => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddFirewallRulesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveFirewallRules => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RemoveFirewallRulesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateFirewallRules => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), UpdateFirewallRulesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::PrepareCanisterMigration => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), PrepareCanisterMigrationPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::CompleteCanisterMigration => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), CompleteCanisterMigrationPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::AddSnsWasm => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddWasmRequest)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetMembership => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), ChangeSubnetMembershipPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSubnetType => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetTypeAssignment => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSnsWasmSnsSubnetIds => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), UpdateSnsSubnetListRequest)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateAllowedPrincipals => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateAllowedPrincipalsRequest)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::RetireReplicaVersion => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RetireReplicaVersionPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::InsertSnsWasmUpgradePathEntries => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), InsertUpgradePathEntriesRequest)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ReviseElectedGuestosVersions => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), ReviseElectedGuestosVersionsPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::BitcoinSetConfig => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), BitcoinSetConfigProposal)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateElectedHostosVersions => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), UpdateElectedHostosVersionsPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodesHostosVersion => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateNodesHostosVersionPayload)?),
                            )?,
                            // Unable to resolve rustls deps when adding `ic-nns-test-utils`
                            ic_nns_governance::pb::v1::NnsFunction::HardResetNnsRootToVersion => "".to_string(),
                            ic_nns_governance::pb::v1::NnsFunction::AddApiBoundaryNodes => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), AddApiBoundaryNodesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveApiBoundaryNodes => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), RemoveApiBoundaryNodesPayload)?))?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateApiBoundaryNodesVersion => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), UpdateApiBoundaryNodesVersionPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToSomeApiBoundaryNodes => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), UpdateApiBoundaryNodesVersionPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToAllUnassignedNodes => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), DeployGuestosToAllUnassignedNodesPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSshReadonlyAccessForAllUnassignedNodes => {
                                serde_json::to_string(
                                    &(Decode!(
                                        a.payload.as_slice(),
                                        UpdateSshReadOnlyAccessForAllUnassignedNodesPayload
                                    )?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ReviseElectedHostosVersions => {
                                serde_json::to_string(
                                    &(Decode!(a.payload.as_slice(), UpdateElectedHostosVersionsPayload)?),
                                )?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployHostosToSomeNodes => serde_json::to_string(
                                &(Decode!(a.payload.as_slice(), UpdateNodesHostosVersionPayload)?),
                            )?,
                            ic_nns_governance::pb::v1::NnsFunction::SubnetRentalRequest => {
                                serde_json::to_string(&(Decode!(a.payload.as_slice(), SubnetRentalRequest)?))?
                            }
                        }
                    }
                }
                Action::ApproveGenesisKyc(a) => serde_json::to_string(&a)?,
                Action::AddOrRemoveNodeProvider(a) => serde_json::to_string(&a)?,
                Action::RewardNodeProvider(a) => serde_json::to_string(&a)?,
                Action::SetDefaultFollowees(a) => serde_json::to_string(&a)?,
                Action::RewardNodeProviders(a) => serde_json::to_string(&a)?,
                Action::RegisterKnownNeuron(a) => serde_json::to_string(&a)?,
                Action::SetSnsTokenSwapOpenTimeWindow(a) => serde_json::to_string(&a)?,
                Action::OpenSnsTokenSwap(a) => serde_json::to_string(&a)?,
                Action::CreateServiceNervousSystem(a) => serde_json::to_string(&a)?,
            },
        })
    }

    type Error = anyhow::Error;
}
