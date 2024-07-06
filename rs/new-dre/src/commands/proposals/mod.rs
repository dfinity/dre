use analyze::Analyze;
use candid::Decode;
use clap::{Args, Subcommand};
use filter::Filter;
use get::Get;
use ic_nervous_system_clients::canister_id_record::CanisterIdRecord;
use ic_nervous_system_root::change_canister::{AddCanisterRequest, ChangeCanisterRequest, StopOrStartCanisterRequest};
use ic_nns_common::types::UpdateIcpXdrConversionRatePayload;

use cycles_minting_canister::SetAuthorizedSubnetworkListArgs;
use ic_nns_governance::{
    governance::{BitcoinSetConfigProposal, SubnetRentalRequest},
    pb::v1::{proposal::Action, ProposalInfo, ProposalStatus, Topic},
};
use ic_protobuf::registry::{
    dc::v1::AddOrRemoveDataCentersProposalPayload, node_operator::v1::RemoveNodeOperatorsPayload,
    node_rewards::v2::UpdateNodeRewardsTableProposalPayload,
};
use ic_sns_wasm::pb::v1::{AddWasmRequest, InsertUpgradePathEntriesRequest, UpdateAllowedPrincipalsRequest, UpdateSnsSubnetListRequest};
use list::List;
use pending::Pending;
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

use super::{ExecutableCommand, RegistryRequirement};

mod analyze;
mod filter;
mod get;
mod list;
mod pending;

#[derive(Args, Debug)]
pub struct Proposals {
    #[clap(subcommand)]
    pub subcommand: ProposalsSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ProposalsSubcommands {
    /// Get list of pending proposals
    Pending(Pending),

    /// Get a proposal by ID
    Get(Get),

    /// Print decentralization change for a CHANGE_SUBNET_MEMBERSHIP proposal given its ID
    Analyze(Analyze),

    /// Better proposal filtering
    Filter(Filter),

    /// List proposals
    List(List),
}

impl ExecutableCommand for Proposals {
    fn require_neuron(&self) -> bool {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.require_neuron(),
            ProposalsSubcommands::Get(g) => g.require_neuron(),
            ProposalsSubcommands::Analyze(a) => a.require_neuron(),
            ProposalsSubcommands::Filter(f) => f.require_neuron(),
            ProposalsSubcommands::List(l) => l.require_neuron(),
        }
    }

    fn require_registry(&self) -> RegistryRequirement {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.require_registry(),
            ProposalsSubcommands::Get(g) => g.require_registry(),
            ProposalsSubcommands::Analyze(a) => a.require_registry(),
            ProposalsSubcommands::Filter(f) => f.require_registry(),
            ProposalsSubcommands::List(l) => l.require_registry(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            ProposalsSubcommands::Pending(p) => p.execute(ctx).await,
            ProposalsSubcommands::Get(g) => g.execute(ctx).await,
            ProposalsSubcommands::Analyze(a) => a.execute(ctx).await,
            ProposalsSubcommands::Filter(f) => f.execute(ctx).await,
            ProposalsSubcommands::List(l) => l.execute(ctx).await,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    id: u64,
    proposer: u64,
    title: String,
    summary: String,
    proposal_timestamp_seconds: u64,
    topic: Topic,
    status: ProposalStatus,
    payload: serde_json::Value,
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
                Action::ManageNeuron(a) => serde_json::to_value(a.command)?,
                Action::ManageNetworkEconomics(a) => serde_json::to_value(a)?,
                Action::Motion(a) => serde_json::to_value(a)?,
                Action::ExecuteNnsFunction(a) => {
                    if a.payload.is_empty() {
                        serde_json::json!({})
                    } else {
                        match a.nns_function() {
                            ic_nns_governance::pb::v1::NnsFunction::Unspecified => serde_json::to_value(a)?,
                            ic_nns_governance::pb::v1::NnsFunction::CreateSubnet => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), CreateSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddNodeToSubnet => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddNodesToSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::NnsCanisterInstall => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddCanisterRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::NnsCanisterUpgrade => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), ChangeCanisterRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::BlessReplicaVersion => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), BlessReplicaVersionPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RecoverSubnet => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RecoverSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateConfigOfSubnet => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AssignNoid => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddNodeOperatorPayload)?)?
                            }
                            // Unable to resolve rustls deps when adding `ic-nns-test-utils`
                            ic_nns_governance::pb::v1::NnsFunction::NnsRootUpgrade => serde_json::json!({}),
                            ic_nns_governance::pb::v1::NnsFunction::IcpXdrConversionRate => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateIcpXdrConversionRatePayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToAllSubnetNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), DeployGuestosToAllSubnetNodesPayload)?)?
                            }
                            // Has an empty payload
                            ic_nns_governance::pb::v1::NnsFunction::ClearProvisionalWhitelist => serde_json::json!({}),
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodesFromSubnet => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RemoveNodesFromSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::SetAuthorizedSubnetworks => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), SetAuthorizedSubnetworkListArgs)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::SetFirewallConfig => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), SetFirewallConfigPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodeOperatorConfig => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateNodeOperatorConfigPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::StopOrStartNnsCanister => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), StopOrStartCanisterRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RemoveNodesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UninstallCode => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), CanisterIdRecord)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodeRewardsTable => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateNodeRewardsTableProposalPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddOrRemoveDataCenters => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddOrRemoveDataCentersProposalPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateUnassignedNodesConfig => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateUnassignedNodesConfigPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveNodeOperators => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RemoveNodeOperatorsPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RerouteCanisterRanges => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RerouteCanisterRangesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddFirewallRules => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddFirewallRulesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveFirewallRules => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RemoveFirewallRulesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateFirewallRules => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateFirewallRulesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::PrepareCanisterMigration => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), PrepareCanisterMigrationPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::CompleteCanisterMigration => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), CompleteCanisterMigrationPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::AddSnsWasm => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddWasmRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetMembership => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), ChangeSubnetMembershipPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSubnetType => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetTypeAssignment => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateSubnetPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSnsWasmSnsSubnetIds => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateSnsSubnetListRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateAllowedPrincipals => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateAllowedPrincipalsRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RetireReplicaVersion => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RetireReplicaVersionPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::InsertSnsWasmUpgradePathEntries => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), InsertUpgradePathEntriesRequest)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ReviseElectedGuestosVersions => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), ReviseElectedGuestosVersionsPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::BitcoinSetConfig => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), BitcoinSetConfigProposal)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateElectedHostosVersions => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateElectedHostosVersionsPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateNodesHostosVersion => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateNodesHostosVersionPayload)?)?
                            }
                            // Unable to resolve rustls deps when adding `ic-nns-test-utils`
                            ic_nns_governance::pb::v1::NnsFunction::HardResetNnsRootToVersion => serde_json::json!({}),
                            ic_nns_governance::pb::v1::NnsFunction::AddApiBoundaryNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), AddApiBoundaryNodesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::RemoveApiBoundaryNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), RemoveApiBoundaryNodesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateApiBoundaryNodesVersion => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateApiBoundaryNodesVersionPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToSomeApiBoundaryNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateApiBoundaryNodesVersionPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployGuestosToAllUnassignedNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), DeployGuestosToAllUnassignedNodesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::UpdateSshReadonlyAccessForAllUnassignedNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateSshReadOnlyAccessForAllUnassignedNodesPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::ReviseElectedHostosVersions => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateElectedHostosVersionsPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::DeployHostosToSomeNodes => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), UpdateNodesHostosVersionPayload)?)?
                            }
                            ic_nns_governance::pb::v1::NnsFunction::SubnetRentalRequest => {
                                serde_json::to_value(Decode!(a.payload.as_slice(), SubnetRentalRequest)?)?
                            }
                        }
                    }
                }
                Action::ApproveGenesisKyc(a) => serde_json::to_value(a)?,
                Action::AddOrRemoveNodeProvider(a) => serde_json::to_value(a)?,
                Action::RewardNodeProvider(a) => serde_json::to_value(a)?,
                Action::SetDefaultFollowees(a) => serde_json::to_value(a)?,
                Action::RewardNodeProviders(a) => serde_json::to_value(a)?,
                Action::RegisterKnownNeuron(a) => serde_json::to_value(a)?,
                Action::SetSnsTokenSwapOpenTimeWindow(a) => serde_json::to_value(a)?,
                Action::OpenSnsTokenSwap(a) => serde_json::to_value(a)?,
                Action::CreateServiceNervousSystem(a) => serde_json::to_value(a)?,
            },
        })
    }

    type Error = anyhow::Error;
}
