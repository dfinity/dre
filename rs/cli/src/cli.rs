use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use ic_base_types::PrincipalId;
use ic_management_types::{Artifact, Network};
use url::Url;

use crate::detect_neuron::Neuron;

// For more info about the version setup, look at https://docs.rs/clap/latest/clap/struct.Command.html#method.version
#[derive(Parser, Clone, Default)]
#[clap(about, version = env!("CARGO_PKG_VERSION"), author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN", global = true)]
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
    #[clap(short, long, env = "YES", global = true, conflicts_with = "simulate")]
    pub yes: bool,

    // Simulate submission of the proposal, but do not actually submit it.
    #[clap(long, aliases = ["dry-run", "dryrun", "no"], global = true, conflicts_with = "yes")]
    pub simulate: bool,

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
        /// NNS subnet id. By default 'tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe'
        #[clap(
            long,
            default_value = "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
        )]
        nns_subnet_id: String,
    },

    /// Manage replica/host-os versions blessing
    #[clap(subcommand)]
    Version(version::Cmd),

    /// Rollout hostos version
    Hostos(hostos::Cmd),

    /// Manage nodes
    Nodes(nodes::Cmd),

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
        #[clap(
            long,
            use_value_delimiter = true,
            value_delimiter = ',',
            value_name = "PROPOSER_ID",
            default_value = "12"
        )]
        accepted_topics: Vec<i32>,
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

    /// Dump registry
    DumpRegistry {
        /// Version to dump. If value is less than 0 will dump the latest version
        #[clap(long, default_value = "-1")]
        version: i64,
        /// Optional path to cached registry
        #[clap(long, env = "LOCAL_REGISTRY_PATH")]
        path: Option<PathBuf>,
    },

    /// Firewall rules
    Firewall {
        #[clap(long, default_value = Some("Proposal to modify firewall rules"))]
        title: Option<String>,
        #[clap(long, default_value = None)]
        summary: Option<String>,
        #[clap(long, default_value = None)]
        motivation: Option<String>,
    },
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

            /// Motivation for resing the subnet
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
        },
    }
}

pub mod version {
    use super::*;

    #[derive(Subcommand, Clone)]
    pub enum Cmd {
        Update(UpdateCmd),
    }

    #[derive(Parser, Clone)]
    pub struct UpdateCmd {
        #[clap(subcommand)]
        pub subcommand: UpdateCommands,
    }

    #[derive(Subcommand, Clone)]
    pub enum UpdateCommands {
        /// Update the elected/blessed replica versions in the registry
        /// by adding a new version and potentially removing obsolete versions
        Replica {
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
    impl From<UpdateCommands> for Artifact {
        fn from(value: UpdateCommands) -> Self {
            match value {
                UpdateCommands::Replica { .. } => Artifact::Replica,
                UpdateCommands::HostOS { .. } => Artifact::HostOs,
            }
        }
    }
}

pub mod hostos {
    use crate::operations::hostos_rollout::{NodeAssignment, NodeOwner};

    use super::*;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }
    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Create a new proposal to rollout an elected HostOS version
        /// to a specified list of nodes
        Rollout {
            version: String,
            /// Node IDs where to rollout the version
            #[clap(long, num_args(1..))]
            nodes: Vec<PrincipalId>,
        },
        /// Select a list of nodes from the registry using node group and
        /// rollout an elected HostOS version to them
        RolloutFromNodeGroup {
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

#[derive(Clone)]
pub struct ParsedCli {
    pub network: Network,
    pub ic_admin_bin_path: Option<String>,
    pub yes: bool,
    pub neuron: Neuron,
}

#[derive(Clone)]
pub struct UpdateVersion {
    pub release_artifact: Artifact,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub update_urls: Vec<String>,
    pub stringified_hash: String,
    pub versions_to_retire: Option<Vec<String>>,
}

impl ParsedCli {
    pub fn get_neuron(&self) -> &Neuron {
        &self.neuron
    }

    pub fn get_update_cmd_args(update_version: &UpdateVersion) -> Vec<String> {
        [
            [
                vec![
                    format!("--{}-version-to-elect", update_version.release_artifact),
                    update_version.version.to_string(),
                    "--release-package-sha256-hex".to_string(),
                    update_version.stringified_hash.to_string(),
                    "--release-package-urls".to_string(),
                ],
                update_version.update_urls.clone(),
            ]
            .concat(),
            match update_version.versions_to_retire.clone() {
                Some(versions) => [
                    vec![format!("--{}-versions-to-unelect", update_version.release_artifact)],
                    versions,
                ]
                .concat(),
                None => vec![],
            },
        ]
        .concat()
    }

    pub async fn from_opts(opts: &Opts, require_authentication: bool) -> anyhow::Result<Self> {
        let network = Network::new(&opts.network, &opts.nns_urls).await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse network from name {} and NNS urls {:?}. Error: {}",
                opts.network,
                opts.nns_urls,
                e
            )
        })?;
        let neuron = Neuron::new(
            &network,
            require_authentication,
            opts.neuron_id,
            opts.private_key_pem.clone(),
            opts.hsm_slot,
            opts.hsm_pin.clone(),
            opts.hsm_key_id.clone(),
        )
        .await?;
        Ok(ParsedCli {
            network,
            yes: opts.yes,
            neuron,
            ic_admin_bin_path: opts.ic_admin.clone(),
        })
    }
}
