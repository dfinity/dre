use crate::cli::version::Commands::{Retire, Update};
use clap::{error::ErrorKind, CommandFactory, Parser};
use ic_canisters::governance_canister_version;
use ic_management_types::requests::NodesRemoveRequest;
use ic_management_types::{MinNakamotoCoefficients, Network, NodeFeature};
use std::collections::BTreeMap;
use std::str::FromStr;

mod cli;
mod clients;
pub(crate) mod defaults;
mod ic_admin;
mod ops_subnet_node_replace;
mod runner;

const STAGING_NEURON_ID: u64 = 49;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logger();
    let mut cli_opts = cli::Opts::parse();
    let mut cmd = cli::Opts::command();

    let governance_canister_v = governance_canister_version(cli_opts.network.get_url()).await?;
    let governance_canister_build = governance_canister_v.stringified_hash;

    ic_admin::with_ic_admin(governance_canister_build.into(), async {
        // Start of actually doing stuff with commands.
        if cli_opts.network == Network::Staging {
            cli_opts.private_key_pem = Some(std::env::var("HOME").expect("Please set HOME env var") + "/.config/dfx/identity/bootstrap-super-leader/identity.pem");
            cli_opts.neuron_id = Some(STAGING_NEURON_ID);
        }

        match &cli_opts.subcommand {
            cli::Commands::DerToPrincipal { path } => {
                let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
                println!("{}", principal);
                Ok(())
            }

            cli::Commands::Subnet(subnet) => {
                let runner = runner::Runner::from_opts(&cli_opts).await?;
                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { .. } | cli::subnet::Commands::Resize { .. } => {
                        if subnet.id.is_none() {
                            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `id` not found")
                                .exit();
                        }
                    }
                    cli::subnet::Commands::Replace {
                        nodes,
                        ..
                    } => {
                        if !nodes.is_empty() && subnet.id.is_some() {
                            cmd.error(
                                ErrorKind::ArgumentConflict,
                                "Both subnet id and a list of nodes to replace are provided. Only one of the two is allowed.",
                            )
                            .exit();
                        } else if nodes.is_empty() && subnet.id.is_none() {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Specify either a subnet id or a list of nodes to replace",
                            )
                            .exit();
                        }
                    }
                    cli::subnet::Commands::Create { .. } => {}
                }

                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { version } => runner.deploy(&subnet.id.unwrap(), version),
                    cli::subnet::Commands::Replace {
                        nodes,
                        no_heal,
                        optimize,
                        motivation,
                        exclude,
                        only,
                        include,
                        min_nakamoto_coefficients,
                        verbose,
                    } => {
                        let min_nakamoto_coefficients = parse_min_nakamoto_coefficients(&mut cmd, min_nakamoto_coefficients);

                            runner
                                .membership_replace(ic_management_types::requests::MembershipReplaceRequest {
                                    target: match &subnet.id {
                                        Some(subnet) => {
                                            ic_management_types::requests::ReplaceTarget::Subnet(*subnet)
                                        }
                                        None => {
                                            if let Some(motivation) = motivation.clone() {
                                                ic_management_types::requests::ReplaceTarget::Nodes {
                                                    nodes: nodes.clone(),
                                                    motivation,
                                                }
                                            } else {
                                                cmd.error(
                                                    ErrorKind::MissingRequiredArgument,
                                                    "Required argument `motivation` not found",
                                                )
                                                .exit();
                                            }
                                        }
                                    },
                                    heal: !no_heal,
                                    optimize: *optimize,
                                    exclude: exclude.clone().into(),
                                    only: only.clone(),
                                    include: include.clone().into(),
                                    min_nakamoto_coefficients,
                                }, *verbose)
                                .await
                    }
                    cli::subnet::Commands::Resize { add, remove, include, only, exclude, motivation, verbose, } => {
                        if let Some(motivation) = motivation.clone() {
                            runner.subnet_resize(ic_management_types::requests::SubnetResizeRequest {
                                subnet: subnet.id.unwrap(),
                                add: *add,
                                remove: *remove,
                                only: only.clone().into(),
                                exclude: exclude.clone().into(),
                                include: include.clone().into(),
                            }, motivation, *verbose).await
                        } else {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Required argument `motivation` not found",
                            )
                            .exit();
                        }
                    }
                    cli::subnet::Commands::Create { size, min_nakamoto_coefficients, exclude, only, include, motivation, verbose, replica_version } => {
                        let min_nakamoto_coefficients = parse_min_nakamoto_coefficients(&mut cmd, min_nakamoto_coefficients);
                        if let Some(motivation) = motivation.clone() {
                            runner.subnet_create(ic_management_types::requests::SubnetCreateRequest {
                                size: *size,
                                min_nakamoto_coefficients,
                                only: only.clone().into(),
                                exclude: exclude.clone().into(),
                                include: include.clone().into(),
                            }, motivation, *verbose, replica_version.clone()).await
                        } else {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Required argument `motivation` not found",
                            )
                            .exit();
                        }
                    }
                }
            }

            cli::Commands::Get { args } => {
                let ic_admin = ic_admin::Cli::from_opts(&cli_opts, false).await?;
                ic_admin.run_passthrough_get(args)
            },

            cli::Commands::Propose { args } => {
                let ic_admin = ic_admin::Cli::from_opts(&cli_opts, true).await?;
                ic_admin.run_passthrough_propose(args)
            },

            cli::Commands::Version(cmd) => {
                match &cmd.subcommand {
                    Retire { edit_summary , simulate} => {
                        let runner = runner::Runner::from_opts(&cli_opts).await?;
                        let (template, versions) = runner.prepare_versions_to_retire(*edit_summary).await?;
                        let ic_admin = ic_admin::Cli::from_opts(&cli_opts, true).await?;
                        ic_admin.propose_run(
                            ic_admin::ProposeCommand::RetireReplicaVersion { versions },
                            ic_admin::ProposeOptions {
                                title: Some("Retire IC replica version".to_string()),
                                summary: Some(template),
                                motivation: None,
                                simulate: *simulate,
                            },
                        )
                    },
                    Update { version, rc_branch_name ,simulate} => {
                        let runner = runner::Runner::from_opts(&cli_opts).await?;
                        let (_, versions) = runner.prepare_versions_to_retire(false).await?;
                        let ic_admin = ic_admin::Cli::from_opts(&cli_opts, true).await?;
                        let new_replica_info = ic_admin::Cli::prepare_to_propose_to_bless_new_replica_version(version, rc_branch_name).await?;
                        let proposal_title = if !versions.is_empty() {
                            Some(format!("Elect new replica binary revision (commit {}), and retire old replica versions {}", version, versions.join(",")))
                        } else {
                            Some(format!("Elect new replica binary revision (commit {})", version))
                        };

                        ic_admin.propose_run(ic_admin::ProposeCommand::UpdateElectedReplicaVersions{
                            version_to_bless: version.to_string(),
                            update_url: new_replica_info.update_url,
                            stringified_hash: new_replica_info.stringified_hash,
                            versions_to_retire: versions.clone(),
                        }, ic_admin::ProposeOptions{
                            title: proposal_title,
                            summary: Some(new_replica_info.summary),
                            motivation: None,
                            simulate: *simulate,
                        })
                    }
                }
            },

            cli::Commands::Nodes(nodes) => {
                match &nodes.subcommand {
                    cli::nodes::Commands::Remove { extra_nodes_filter, no_auto, motivation } => {
                        if motivation.is_none() && !extra_nodes_filter.is_empty() {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Required argument `motivation` not found",
                            )
                            .exit();
                        }
                        let runner = runner::Runner::from_opts(&cli_opts).await?;
                        runner.remove_nodes(NodesRemoveRequest {
                            extra_nodes_filter: extra_nodes_filter.clone(),
                            no_auto: *no_auto,
                            motivation: motivation.clone().unwrap_or_default(),
                        }).await
                    },
                }
            },
        }
    })
    .await
}

// Construct MinNakamotoCoefficients from an array (slice) of ["key=value"], and
// prefill default values
//
// Examples:
// [] => use defaults
//           -> "node_provider" Nakamoto Coefficient (NC) needs to be >= 5.0
//           -> average NC >= 3.0
//
// ["node_provider=4"] => override the "node_provider" NC
//           -> "node_provider" NC >= 4.0
//           -> average NC >= 3.0 (default)
//
// ["node_provider=4", "average=2"] => override "node_provider" and average NC
//           -> "node_provider" NC >= 4.0
//           -> average NC >= 2.0
//
// ["data_centers=4"] => override the "data_centers" NC
//           -> "data_centers" NC >= 4.0
//           -> "node_provider" NC >= 5.0 (default)
//           -> average NC >= 3.0 (default)
fn parse_min_nakamoto_coefficients(
    cmd: &mut clap::Command,
    min_nakamoto_coefficients: &[String],
) -> Option<MinNakamotoCoefficients> {
    let min_nakamoto_coefficients: Vec<String> = if min_nakamoto_coefficients.is_empty() {
        ["node_provider=5", "average=3"]
            .iter()
            .map(|s| String::from(*s))
            .collect()
    } else {
        min_nakamoto_coefficients.to_vec()
    };

    let mut average = 3.0;
    let min_nakamoto_coefficients = min_nakamoto_coefficients
        .iter()
        .filter_map(|s| {
            let (key, val) = match s.split_once('=') {
                Some(s) => s,
                None => cmd
                    .error(ErrorKind::ValueValidation, "Value requires exactly one '=' symbol")
                    .exit(),
            };
            if key.to_lowercase() == "average" {
                average = val
                    .parse::<f64>()
                    .map_err(|_| {
                        cmd.error(ErrorKind::ValueValidation, "Failed to parse feature from string")
                            .exit()
                    })
                    .unwrap();
                None
            } else {
                let feature = match NodeFeature::from_str(key) {
                    Ok(v) => v,
                    Err(_) => cmd
                        .error(ErrorKind::ValueValidation, "Failed to parse feature from string")
                        .exit(),
                };
                let val: f64 = val
                    .parse::<f64>()
                    .map_err(|_| {
                        cmd.error(ErrorKind::ValueValidation, "Failed to parse feature from string")
                            .exit()
                    })
                    .unwrap();
                Some((feature, val))
            }
        })
        .collect::<BTreeMap<NodeFeature, f64>>();

    Some(MinNakamotoCoefficients {
        coefficients: min_nakamoto_coefficients,
        average,
    })
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Set a default logging level: info, if nothing else specified in environment
                // variables RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
