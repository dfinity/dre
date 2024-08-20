use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_common::pb::v1::ProposalId;
use itertools::Itertools;
use log::{error, info, warn};
use strum::IntoEnumIterator;

use std::fmt::Display;

use clap::ValueEnum;
use ic_nns_governance::pb::v1::{ListProposalInfo, ProposalStatus as ProposalStatusUpstream, Topic as TopicUpstream};

use crate::commands::{proposals::Proposal, ExecutableCommand, IcAdminRequirement};
#[derive(Args, Debug)]
pub struct Filter {
    /// Limit on the number of \[ProposalInfo\] to return. If value greater than
    /// canister limit is used it will still be fetch the wanted number of proposals.
    #[clap(long, default_value = "100")]
    pub limit: u32,

    /// Proposal statuses to include. If not specified will include all proposals.
    #[arg(value_enum)]
    #[clap(long, aliases = ["status"], short = 's')]
    pub statuses: Vec<ProposalStatus>,

    /// Proposal topics to include. If not specified will include all proposals.
    #[arg(value_enum)]
    #[clap(long, aliases = ["topic"], short = 't')]
    pub topics: Vec<Topic>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ProposalStatus {
    Unspecified = 0,
    /// A decision (adopt/reject) has yet to be made.
    Open = 1,
    /// The proposal has been rejected.
    Rejected = 2,
    /// The proposal has been adopted (sometimes also called
    /// "accepted"). At this time, either execution as not yet started,
    /// or it has but the outcome is not yet known.
    Adopted = 3,
    /// The proposal was adopted and successfully executed.
    Executed = 4,
    /// The proposal was adopted, but execution failed.
    Failed = 5,
}

impl Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ProposalStatusUpstream::from(self.clone()).as_str_name())
    }
}

impl From<ProposalStatus> for ProposalStatusUpstream {
    fn from(value: ProposalStatus) -> Self {
        match value {
            ProposalStatus::Unspecified => Self::Unspecified,
            ProposalStatus::Open => Self::Open,
            ProposalStatus::Rejected => Self::Rejected,
            ProposalStatus::Adopted => Self::Adopted,
            ProposalStatus::Executed => Self::Executed,
            ProposalStatus::Failed => Self::Failed,
        }
    }
}

impl From<ProposalStatusUpstream> for ProposalStatus {
    fn from(value: ProposalStatusUpstream) -> Self {
        match value {
            ProposalStatusUpstream::Unspecified => Self::Unspecified,
            ProposalStatusUpstream::Open => Self::Open,
            ProposalStatusUpstream::Rejected => Self::Rejected,
            ProposalStatusUpstream::Adopted => Self::Adopted,
            ProposalStatusUpstream::Executed => Self::Executed,
            ProposalStatusUpstream::Failed => Self::Failed,
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Topic {
    /// The `Unspecified` topic is used as a fallback when
    /// following. That is, if no followees are specified for a given
    /// topic, the followees for this topic are used instead.
    Unspecified = 0,
    /// A special topic by means of which a neuron can be managed by the
    /// followees for this topic (in this case, there is no fallback to
    /// 'unspecified'). Votes on this topic are not included in the
    /// voting history of the neuron (cf., `recent_ballots` in `Neuron`).
    ///
    /// For proposals on this topic, only followees on the 'neuron
    /// management' topic of the neuron that the proposals pertains to
    /// are allowed to vote.
    ///
    /// As the set of eligible voters on this topic is restricted,
    /// proposals on this topic have a *short voting period*.
    NeuronManagement = 1,
    /// All proposals that provide “real time” information about the
    /// value of ICP, as measured by an IMF SDR, which allows the NNS to
    /// convert ICP to cycles (which power computation) at a rate which
    /// keeps their real world cost constant. Votes on this topic are not
    /// included in the voting history of the neuron (cf.,
    /// `recent_ballots` in `Neuron`).
    ///
    /// Proposals on this topic have a *short voting period* due to their
    /// frequency.
    ExchangeRate = 2,
    /// All proposals that administer network economics, for example,
    /// determining what rewards should be paid to node operators.
    NetworkEconomics = 3,
    /// All proposals that administer governance, for example to freeze
    /// malicious canisters that are harming the network.
    Governance = 4,
    /// All proposals that administer node machines, including, but not
    /// limited to, upgrading or configuring the OS, upgrading or
    /// configuring the virtual machine framework and upgrading or
    /// configuring the node replica software.
    NodeAdmin = 5,
    /// All proposals that administer network participants, for example,
    /// granting and revoking DCIDs (data center identities) or NOIDs
    /// (node operator identities).
    ParticipantManagement = 6,
    /// All proposals that administer network subnets, for example
    /// creating new subnets, adding and removing subnet nodes, and
    /// splitting subnets.
    SubnetManagement = 7,
    /// Installing and upgrading “system” canisters that belong to the network.
    /// For example, upgrading the NNS.
    NetworkCanisterManagement = 8,
    /// Proposals that update KYC information for regulatory purposes,
    /// for example during the initial Genesis distribution of ICP in the
    /// form of neurons.
    Kyc = 9,
    /// Topic for proposals to reward node providers.
    NodeProviderRewards = 10,
    /// IC OS upgrade proposals
    /// -----------------------
    /// ICP runs on a distributed network of nodes grouped into subnets. Each node runs a stack of
    /// operating systems, including HostOS (runs on bare metal) and GuestOS (runs inside HostOS;
    /// contains, e.g., the ICP replica process). HostOS and GuestOS are distributed via separate disk
    /// images. The umbrella term IC OS refers to the whole stack.
    ///
    /// The IC OS upgrade process involves two phases, where the first phase is the election of a new
    /// IC OS version and the second phase is the deployment of a previously elected IC OS version on
    /// all nodes of a subnet or on some number of nodes (including nodes comprising subnets and
    /// unassigned nodes).
    ///
    /// A special case is for API boundary nodes, special nodes that route API requests to a replica
    /// of the right subnet. API boundary nodes run a different process than the replica, but their
    /// executable is distributed via the same disk image as GuestOS. Therefore, electing a new GuestOS
    /// version also results in a new version of boundary node software being elected.
    ///
    /// Proposals handling the deployment of IC OS to some nodes. It is possible to deploy only
    /// the versions of IC OS that are in the set of elected IC OS versions.
    IcOsVersionDeployment = 12,
    /// Proposals for changing the set of elected IC OS versions.
    IcOsVersionElection = 13,
    /// Proposals related to SNS and Community Fund.
    SnsAndCommunityFund = 14,
    /// Proposals related to the management of API Boundary Nodes
    ApiBoundaryNodeManagement = 15,
    /// Proposals related to the management of API Boundary Nodes
    SubnetRental = 16,
    /// Proposals to manage protocol canisters. Those are canisters that are considered part of the IC
    /// protocol, without which the IC will not be able to function properly.
    ProtocolCanisterManagement = 17,
    /// Proposals related to Service Nervous System (SNS) - (1) upgrading SNS-W, (2) upgrading SNS
    /// Aggregator, and (3) adding WASM's or custom upgrade paths to SNS-W.
    ServiceNervousSystemManagement = 18,
}

impl From<Topic> for TopicUpstream {
    fn from(value: Topic) -> Self {
        match value {
            Topic::Unspecified => Self::Unspecified,
            Topic::NeuronManagement => Self::NeuronManagement,
            Topic::ExchangeRate => Self::ExchangeRate,
            Topic::NetworkEconomics => Self::NetworkEconomics,
            Topic::Governance => Self::Governance,
            Topic::NodeAdmin => Self::NodeAdmin,
            Topic::ParticipantManagement => Self::ParticipantManagement,
            Topic::SubnetManagement => Self::SubnetManagement,
            Topic::NetworkCanisterManagement => Self::NetworkCanisterManagement,
            Topic::Kyc => Self::Kyc,
            Topic::NodeProviderRewards => Self::NodeProviderRewards,
            Topic::IcOsVersionDeployment => Self::IcOsVersionDeployment,
            Topic::IcOsVersionElection => Self::IcOsVersionElection,
            Topic::SnsAndCommunityFund => Self::SnsAndCommunityFund,
            Topic::ApiBoundaryNodeManagement => Self::ApiBoundaryNodeManagement,
            Topic::SubnetRental => Self::SubnetRental,
            Topic::ProtocolCanisterManagement => Self::ProtocolCanisterManagement,
            Topic::ServiceNervousSystemManagement => Self::ServiceNervousSystemManagement,
        }
    }
}

impl From<TopicUpstream> for Topic {
    fn from(value: TopicUpstream) -> Self {
        match value {
            TopicUpstream::Unspecified => Self::Unspecified,
            TopicUpstream::NeuronManagement => Self::NeuronManagement,
            TopicUpstream::ExchangeRate => Self::ExchangeRate,
            TopicUpstream::NetworkEconomics => Self::NetworkEconomics,
            TopicUpstream::Governance => Self::Governance,
            TopicUpstream::NodeAdmin => Self::NodeAdmin,
            TopicUpstream::ParticipantManagement => Self::ParticipantManagement,
            TopicUpstream::SubnetManagement => Self::SubnetManagement,
            TopicUpstream::NetworkCanisterManagement => Self::NetworkCanisterManagement,
            TopicUpstream::Kyc => Self::Kyc,
            TopicUpstream::NodeProviderRewards => Self::NodeProviderRewards,
            TopicUpstream::IcOsVersionDeployment => Self::IcOsVersionDeployment,
            TopicUpstream::IcOsVersionElection => Self::IcOsVersionElection,
            TopicUpstream::SnsAndCommunityFund => Self::SnsAndCommunityFund,
            TopicUpstream::ApiBoundaryNodeManagement => Self::ApiBoundaryNodeManagement,
            TopicUpstream::SubnetRental => Self::SubnetRental,
            TopicUpstream::ProtocolCanisterManagement => Self::ProtocolCanisterManagement,
            TopicUpstream::ServiceNervousSystemManagement => Self::ServiceNervousSystemManagement,
        }
    }
}

impl ExecutableCommand for Filter {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);

        let exclude_topic = match self.topics.is_empty() {
            true => vec![],
            false => {
                let mut all_topics = TopicUpstream::iter().collect_vec();
                for topic in self.topics.iter().map(|f| f.clone().into()).collect::<Vec<TopicUpstream>>() {
                    all_topics.retain(|t| *t != topic);
                }
                all_topics
            }
        };
        let statuses = self.statuses.iter().map(|f| f.clone().into()).collect::<Vec<ProposalStatusUpstream>>();

        let mut remaining = self.limit;
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
            self.limit,
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
                warn!("No more proposals available and there is {} remaining to find", remaining);
                break;
            }
        }

        println!("{}", serde_json::to_string_pretty(&proposals)?);
        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
