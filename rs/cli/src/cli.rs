use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use humantime::parse_duration;
use ic_base_types::PrincipalId;
use ic_management_types::Artifact;
use ic_registry_keys::FirewallRulesScope;
use std::{path::PathBuf, time::Duration};
use url::Url;

// For more info about the version setup, look at https://docs.rs/clap/latest/clap/struct.Command.html#method.version
#[derive(Parser, Clone, Default)]
#[clap(about, version = env!("CARGO_PKG_VERSION"), author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN", global = true, hide_env_values = true)]
    pub hsm_pin: Option<String>,
    #[clap(long, value_parser=maybe_hex::<u64>, env = "HSM_SLOT", global = true)]
    pub hsm_slot: Option<u64>,
    #[clap(long, env = "HSM_KEY_ID", global = true)]
    pub hsm_key_id: Option<String>,
    #[clap(long, env = "PRIVATE_KEY_PEM", global = true)]
    pub private_key_pem: Option<String>,
    #[clap(long, env = "NEURON_ID", global = true)]
    pub neuron_id: Option<u64>,
    #[clap(long, env = "IC_ADMIN", global = true)]
    pub ic_admin: Option<String>,
    #[clap(long, env = "DEV", global = true)]
    pub dev: bool,

    // Skip the confirmation prompt
    #[clap(short, long, env = "YES", global = true, conflicts_with = "dry_run")]
    pub yes: bool,

    // Dry-run or simulate proposal submission, but do not actually submit it.
    // Will show the ic-admin command and the proposal Payload
    #[clap(long, aliases = ["dry-run", "dryrun", "simulate", "no"], global = true, conflicts_with = "yes")]
    pub dry_run: bool,

    #[clap(long, env = "VERBOSE", global = true)]
    pub verbose: bool,

    // Specify the target network: "mainnet" (default), "staging", or a testnet name
    #[clap(long, env = "NETWORK", default_value = "mainnet")]
    pub network: String,

    // NNS_URLs for the target network, comma separated.
    // The argument is mandatory for testnets, and is optional for mainnet and staging
    #[clap(long, env = "NNS_URLS", aliases = &["registry-url", "nns-url"], value_delimiter = ',')]
    pub nns_urls: Vec<Url>,

    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    // Convert a DER file to a Principal
    DerToPrincipal {
        /// Path to the DER file
        path: String,
    },

    Heal {
        /// Max number of nodes to be replaced per subnet.
        /// Optimization will be performed automatically maximizing the decentralization
        /// and minimizing the number of replaced nodes per subnet
        #[clap(short, long)]
        max_replaceable_nodes_per_sub: Option<usize>,
    },

    /// Manage an existing subnet
    Subnet(subnet::Cmd),
    /// Get a value using ic-admin CLI
    Get {
        /// Arbitrary ic-admin args
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Place a proposal using the ic-admin CLI
    Propose {
        /// Arbitrary ic-admin args
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Place a proposal for updating unassigned nodes config
    UpdateUnassignedNodes {
        /// NNS subnet id
        #[clap(long)]
        nns_subnet_id: Option<String>,
    },

    /// Manage replica/host-os versions blessing
    #[clap(subcommand)]
    Version(version::Cmd),

    /// Rollout hostos version
    Hostos(hostos::Cmd),

    /// Manage nodes
    Nodes(nodes::Cmd),

    /// Manage API boundary nodes
    ApiBoundaryNodes(api_boundary_nodes::Cmd),

    /// Vote on our proposals
    Vote {
        /// Override default accepted proposers
        /// These are the proposers which proposals will
        /// be automatically voted on
        ///
        /// By default: DRE + automation neuron 80
        #[clap(
            long,
            use_value_delimiter = true,
            value_delimiter = ',',
            value_name = "PROPOSER_ID",
            default_value = "80,39,40,46,58,61,77"
        )]
        accepted_neurons: Vec<u64>,

        /// Override default topics to vote on
        /// Use with caution! This is subcommand is intended to be used
        /// only by DRE in processes of rolling out new versions,
        /// everything else should be double checked manually
        ///
        /// By default: SubnetReplicaVersionManagement
        #[clap(long, use_value_delimiter = true, value_delimiter = ',', value_name = "PROPOSER_ID", default_value = "12")]
        accepted_topics: Vec<i32>,

        /// Override default sleep time
        #[clap(long, default_value = "60s", value_parser = parse_duration)]
        sleep_time: Duration,
    },

    /// Trustworthy Metrics
    TrustworthyMetrics {
        /// Wallet that should be used to query node metrics history
        /// in form of canister id
        wallet: String,

        /// Start at timestamp in nanoseconds
        start_at_timestamp: u64,

        /// Vector of subnets to query, if empty will dump metrics for
        /// all subnets
        subnet_ids: Vec<PrincipalId>,
    },

    /// Registry inspection (dump) operations
    Registry {
        /// Version to dump. If value is less than 0 will dump the latest version
        #[clap(long, default_value = "-1")]
        version: i64,

        /// Output file (default is stdout)
        #[clap(short = 'o', long)]
        output: Option<PathBuf>,

        /// Output only information related to the node operator records with incorrect rewards
        #[clap(long)]
        incorrect_rewards: bool,

        /// Optional path to cached registry, can be used to inspect an arbitrary path
        #[clap(long, env = "LOCAL_REGISTRY_PATH")]
        local_registry_path: Option<PathBuf>,
    },

    /// Firewall rules
    Firewall {
        #[clap(long, default_value = Some("Proposal to modify firewall rules"))]
        title: Option<String>,
        #[clap(long, default_value = None, required = true)]
        summary: Option<String>,
        /// Ruleset scope: "global", "replica_nodes", "api_boundary_nodes", "subnet(SUBNET_ID)", "node(NODE_ID)"
        #[clap(long, default_value = None, required = true)]
        rules_scope: FirewallRulesScope,
    },

    /// Proposal Listing
    Proposals(proposals::Cmd),
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Get { args: vec![] }
    }
}

pub mod subnet {
    use super::*;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(long, short)]
        pub id: Option<PrincipalId>,
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Create a new proposal to rollout a new version to the subnet
        Deploy { version: String },

        /// Replace the nodes in a subnet
        Replace {
            /// Set of custom nodes to be replaced
            nodes: Vec<PrincipalId>,

            /// Do not replace unhealthy nodes
            #[clap(long)]
            no_heal: bool,

            /// Amount of nodes to be replaced by decentralization optimization
            /// algorithm
            #[clap(short, long)]
            optimize: Option<usize>,

            /// Motivation for replacing custom nodes
            #[clap(short, long, aliases = ["summary"])]
            motivation: Option<String>,

            /// Minimum Nakamoto coefficients after the replacement
            #[clap(long, num_args(1..))]
            min_nakamoto_coefficients: Vec<String>,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..))]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,
        },

        /// Resize the subnet
        Resize {
            // Number of nodes to be added
            #[clap(long)]
            add: usize,

            // Number of nodes to be removed
            #[clap(long)]
            remove: usize,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..))]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,

            /// Motivation for resizing the subnet
            #[clap(short, long, aliases = ["summary"])]
            motivation: Option<String>,
        },

        /// Create a new subnet
        Create {
            /// Number of nodes in the subnet
            #[clap(long, default_value_t = 13)]
            size: usize,

            /// Minimum nakamoto coefficients desired
            #[clap(long, num_args(1..))]
            min_nakamoto_coefficients: Vec<String>,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..))]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,

            /// Motivation for creating the subnet
            #[clap(short, long, aliases = ["summary"])]
            motivation: Option<String>,

            #[clap(long)]
            replica_version: Option<String>,

            /// Arbitrary other ic-admin args
            #[clap(allow_hyphen_values = true)]
            other_args: Vec<String>,

            /// Provide the list of all arguments that ic-admin accepts for subnet creation
            #[clap(long)]
            help_other_args: bool,
        },

        /// Replace all nodes in a subnet
        Rescue {
            /// Node features or Node IDs to exclude from the replacement
            #[clap(long, num_args(1..))]
            keep_nodes: Option<Vec<String>>,
        },
    }
}

pub mod version {
    use super::*;

    #[derive(Subcommand, Clone)]
    pub enum Cmd {
        ReviseElectedVersions(ReviseElectedVersionsCmd),
    }

    #[derive(Parser, Clone)]
    pub struct ReviseElectedVersionsCmd {
        #[clap(subcommand)]
        pub subcommand: ReviseElectedVersionsCommands,
    }

    #[derive(Subcommand, Clone)]
    pub enum ReviseElectedVersionsCommands {
        /// Update the elected/blessed GuestOS versions in the registry
        /// by adding a new version and potentially removing obsolete versions
        GuestOS {
            /// Specify the commit hash of the version that is being elected.
            version: String,

            /// Git tag for the release.
            release_tag: String,

            /// Force proposal submission, ignoring missing download URLs
            #[clap(long)]
            force: bool,
        },
        /// Update the elected/blessed HostOS versions in the registry
        /// by adding a new version and potentially removing obsolete versions
        HostOS {
            /// Specify the commit hash of the version that is being elected.
            version: String,

            /// Git tag for the release.
            release_tag: String,

            /// Force proposal submission, ignoring missing download URLs
            #[clap(long)]
            force: bool,
        },
    }
    impl From<ReviseElectedVersionsCommands> for Artifact {
        fn from(value: ReviseElectedVersionsCommands) -> Self {
            match value {
                ReviseElectedVersionsCommands::GuestOS { .. } => Artifact::GuestOs,
                ReviseElectedVersionsCommands::HostOS { .. } => Artifact::HostOs,
            }
        }
    }
}

pub mod hostos {
    #[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Parser, Default)]
    pub enum NodeOwner {
        Dfinity,
        Others,
        #[default]
        All,
    }

    impl std::fmt::Display for NodeOwner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                NodeOwner::Dfinity => write!(f, "DFINITY"),
                NodeOwner::Others => write!(f, "External"),
                NodeOwner::All => write!(f, "DFINITY+External"),
            }
        }
    }

    #[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default)]
    pub enum NodeAssignment {
        Unassigned,
        Assigned,
        #[default]
        All,
    }

    impl std::fmt::Display for NodeAssignment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                NodeAssignment::Unassigned => write!(f, "Unassigned"),
                NodeAssignment::Assigned => write!(f, "In Subnet"),
                NodeAssignment::All => write!(f, "In Subnet+Unassigned"),
            }
        }
    }

    use super::*;
    use clap::ValueEnum;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }
    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Roll out an elected HostOS version to the specified list of nodes.
        /// The provided "version" must be already elected.
        /// The "nodes" list must contain the node IDs where the version should be rolled out.
        Rollout {
            #[clap(long, required = true)]
            version: String,
            /// Node IDs where to rollout the version
            #[clap(long, num_args(1..), required = true)]
            nodes: Vec<PrincipalId>,
        },
        /// Smarter roll out of the elected HostOS version to groups of nodes.
        /// The groups of nodes are created based on assignment to subnets, and on the owner of the nodes: DFINITY/other.
        /// The provided "version" must be already elected.
        RolloutFromNodeGroup {
            #[clap(long, required = true)]
            version: String,
            /// Specify if the group of nodes considered for the rollout should be assigned on
            /// a subnet or not
            #[arg(value_enum)]
            #[clap(long)]
            assignment: Option<NodeAssignment>,
            /// Owner of the group of nodes considered for the rollout
            #[arg(value_enum)]
            #[clap(long)]
            owner: Option<NodeOwner>,
            /// Specifies the filter used to exclude from the update a set of nodes
            #[clap(long, num_args(1..))]
            exclude: Option<Vec<PrincipalId>>,
            /// How many nodes in the group to update with the version specified
            /// supported values are absolute numbers (10) or percentage (10%)
            #[clap(long)]
            nodes_in_group: String,
        },
    }
}

pub mod nodes {
    use super::*;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Remove the nodes from the network
        Remove {
            /// Skip removal of duplicate or dead nodes
            #[clap(long)]
            no_auto: bool,

            /// Remove also degraded nodes; by default only dead (offline) nodes are automatically removed
            #[clap(long)]
            remove_degraded: bool,

            /// Specifies the filter used to remove extra nodes
            extra_nodes_filter: Vec<String>,

            /// Features or Node IDs to not remove (exclude from the removal)
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Motivation for removing additional nodes
            #[clap(long, aliases = ["summary"])]
            motivation: Option<String>,
        },
    }
}

pub mod api_boundary_nodes {
    use super::*;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Update specified set of nodes to the provided version.
        /// The provided "version" must be already elected.
        /// The "nodes" list must contain the node IDs where the version should be rolled out.
        Update {
            /// Node IDs where to rollout the version
            #[clap(long, num_args(1..), required = true)]
            nodes: Vec<PrincipalId>,

            #[clap(long, required = true)]
            version: String,

            /// Motivation for creating the subnet
            #[clap(short, long, aliases = ["summary"], required = true)]
            motivation: Option<String>,
        },

        /// Turn a set of unassigned nodes into API BNs
        Add {
            /// Node IDs to turn into API BNs
            #[clap(long, num_args(1..), required = true)]
            nodes: Vec<PrincipalId>,

            /// guestOS version
            #[clap(long, required = true)]
            version: String,

            /// Motivation for creating the subnet
            #[clap(short, long, aliases = ["summary"], required = true)]
            motivation: Option<String>,
        },

        /// Decommission a set of API BNs and turn them again in unassigned nodes
        Remove {
            /// Node IDs of API BNs that should be turned into unassigned nodes again
            #[clap(long, num_args(1..), required = true)]
            nodes: Vec<PrincipalId>,

            /// Motivation for removing the API BNs
            #[clap(short, long, aliases = ["summary"], required = true)]
            motivation: Option<String>,
        },
    }
}

pub mod proposals {
    use std::fmt::Display;

    use clap::ValueEnum;
    use ic_nns_governance::pb::v1::{ProposalStatus as ProposalStatusUpstream, Topic as TopicUpstream};

    use super::*;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Get list of pending proposals
        Pending,

        /// Get list of filtered proposals
        List {
            /// Limit on the number of \[ProposalInfo\] to return. If no value is
            /// specified, or if a value greater than 100 is specified, 100
            /// will be used.
            #[clap(long, default_value = "100")]
            limit: u32,
            /// If specified, only return proposals that are strictly earlier than
            /// the specified proposal according to the proposal ID. If not
            /// specified, start with the most recent proposal.
            #[clap(long)]
            before_proposal: Option<u64>,
            /// Exclude proposals with a topic in this list. This is particularly
            /// useful to exclude proposals on the topics TOPIC_EXCHANGE_RATE and
            /// TOPIC_KYC which most users are not likely to be interested in
            /// seeing.
            #[clap(long)]
            exclude_topic: Vec<i32>,
            /// Include proposals that have a reward status in this list (see
            /// \[ProposalRewardStatus\] for more information). If this list is
            /// empty, no restriction is applied. For example, many users listing
            /// proposals will only be interested in proposals for which they can
            /// receive voting rewards, i.e., with reward status
            /// PROPOSAL_REWARD_STATUS_ACCEPT_VOTES.
            #[clap(long)]
            include_reward_status: Vec<i32>,
            /// Include proposals that have a status in this list (see
            /// \[ProposalStatus\] for more information). If this list is empty, no
            /// restriction is applied.
            #[clap(long)]
            include_status: Vec<i32>,
            /// Include all ManageNeuron proposals regardless of the visibility of the
            /// proposal to the caller principal. Note that exclude_topic is still
            /// respected even when this option is set to true.
            #[clap(long)]
            include_all_manage_neuron_proposals: Option<bool>,
            /// Omits "large fields" from the response. Currently only omits the
            /// `logo` and `token_logo` field of CreateServiceNervousSystem proposals. This
            /// is useful to improve download times and to ensure that the response to the
            /// request doesn't exceed the message size limit.
            #[clap(long)]
            omit_large_fields: Option<bool>,
        },

        /// More user-friendly command for filtering proposals
        Filter {
            /// Limit on the number of \[ProposalInfo\] to return. If value greater than
            /// canister limit is used it will still be fetch the wanted number of proposals.
            #[clap(long, default_value = "100")]
            limit: u32,

            /// Proposal statuses to include. If not specified will include all proposals.
            #[arg(value_enum)]
            #[clap(long, aliases = ["status"], short = 's')]
            statuses: Vec<ProposalStatus>,

            /// Proposal topics to include. If not specified will include all proposals.
            #[arg(value_enum)]
            #[clap(long, aliases = ["topic"], short = 't')]
            topics: Vec<Topic>,
        },

        /// Get a proposal by ID
        Get {
            /// Proposal ID
            proposal_id: u64,
        },

        /// Print decentralization change for a CHANGE_SUBNET_MEMBERSHIP proposal gived its ID
        Analyze {
            /// Proposal ID
            proposal_id: u64,
        },
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
            }
        }
    }
}
