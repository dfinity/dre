use crate::general::{get_node_metrics_history, vote_on_proposals};
use crate::ic_admin::IcAdminWrapper;
use clap::{error::ErrorKind, CommandFactory, Parser};
use dotenv::dotenv;
use ic_base_types::CanisterId;
use ic_canisters::governance::governance_canister_version;
use ic_management_backend::endpoints;
use ic_management_types::requests::NodesRemoveRequest;
use ic_management_types::{Artifact, MinNakamotoCoefficients, Network, NodeFeature, NodeGroupUpdate, NumberOfNodes};
use log::info;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

mod cli;
mod clients;
pub(crate) mod defaults;
mod detect_neuron;
mod general;
mod ic_admin;
mod ops_subnet_node_replace;
mod registry_dump;
mod runner;

const STAGING_NEURON_ID: u64 = 49;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    init_logger();
    info!("Running version {}", env!("CARGO_PKG_VERSION"));

    let mut cli_opts = cli::Opts::parse();
    let mut cmd = cli::Opts::command();

    let governance_canister_v = governance_canister_version(cli_opts.network.get_url()).await?;
    let governance_canister_version = governance_canister_v.stringified_hash;

    let target_network = cli_opts.network.clone();
    let (tx, rx) = mpsc::channel();

    let backend_port = local_unused_port();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            endpoints::run_backend(target_network, "127.0.0.1", backend_port, true, Some(tx))
                .await
                .expect("failed")
        });
    });
    let srv = rx.recv().unwrap();

    ic_admin::with_ic_admin(governance_canister_version.into(), async {

        // Start of actually doing stuff with commands.
        if cli_opts.network == Network::Staging {
            cli_opts.private_key_pem = Some(std::env::var("HOME").expect("Please set HOME env var") + "/.config/dfx/identity/bootstrap-super-leader/identity.pem");
            cli_opts.neuron_id = Some(STAGING_NEURON_ID);
        }

        let simulate = cli_opts.simulate;

        match &cli_opts.subcommand {
            cli::Commands::DerToPrincipal { path } => {
                let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
                println!("{}", principal);
                Ok(())
            }

            cli::Commands::Subnet(subnet) => {
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
                    cli::subnet::Commands::Deploy { version } => {
                        let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, false).await?.into(), backend_port).await?;
                        runner.deploy(&subnet.id.unwrap(), version, simulate)
                    },
                    cli::subnet::Commands::Replace {
                        nodes,
                        no_heal,
                        optimize,
                        motivation,
                        exclude,
                        only,
                        include,
                        min_nakamoto_coefficients,
                    } => {
                        let min_nakamoto_coefficients = parse_min_nakamoto_coefficients(&mut cmd, min_nakamoto_coefficients);
                            let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
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
                                }, cli_opts.verbose, simulate)
                                .await
                    }
                    cli::subnet::Commands::Resize { add, remove, include, only, exclude, motivation, } => {
                        if let Some(motivation) = motivation.clone() {
                            let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                            runner.subnet_resize(ic_management_types::requests::SubnetResizeRequest {
                                subnet: subnet.id.unwrap(),
                                add: *add,
                                remove: *remove,
                                only: only.clone().into(),
                                exclude: exclude.clone().into(),
                                include: include.clone().into(),
                            }, motivation, cli_opts.verbose, simulate).await
                        } else {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Required argument `motivation` not found",
                            )
                            .exit();
                        }
                    }
                    cli::subnet::Commands::Create { size, min_nakamoto_coefficients, exclude, only, include, motivation, replica_version } => {
                        let min_nakamoto_coefficients = parse_min_nakamoto_coefficients(&mut cmd, min_nakamoto_coefficients);
                        if let Some(motivation) = motivation.clone() {
                            let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                            runner.subnet_create(ic_management_types::requests::SubnetCreateRequest {
                                size: *size,
                                min_nakamoto_coefficients,
                                only: only.clone().into(),
                                exclude: exclude.clone().into(),
                                include: include.clone().into(),
                            }, motivation, cli_opts.verbose, simulate, replica_version.clone()).await
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
                let ic_admin: IcAdminWrapper = cli::Cli::from_opts(&cli_opts, false).await?.into();
                ic_admin.run_passthrough_get(args)
            },

            cli::Commands::Propose { args } => {
                let ic_admin: IcAdminWrapper = cli::Cli::from_opts(&cli_opts, true).await?.into();
                ic_admin.run_passthrough_propose(args, simulate)
            },

            cli::Commands::UpdateUnassignedNodes { nns_subnet_id } => {
                let ic_admin: IcAdminWrapper = cli::Cli::from_opts(&cli_opts, true).await?.into();
                ic_admin.update_unassigned_nodes( nns_subnet_id, cli_opts.network, simulate).await
            },

            cli::Commands::Version(version_command) => {
                match &version_command {
                    cli::version::Cmd::Update(update_command) => {
                        let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                        let ic_admin: IcAdminWrapper = cli::Cli::from_opts(&cli_opts, true).await?.into();
                        let release_artifact: &Artifact = &update_command.subcommand.clone().into();

                        let update_version = match &update_command.subcommand {
                            cli::version::UpdateCommands::Replica { version, release_tag} | cli::version::UpdateCommands::HostOS { version, release_tag} => {
                                ic_admin::IcAdminWrapper::prepare_to_propose_to_update_elected_versions(
                                    release_artifact,
                                    version,
                                    release_tag,
                                    runner.prepare_versions_to_retire(release_artifact, false).await.map(|res| res.1)?,
                                )
                            }
                        }.await?;

                        ic_admin.propose_run(ic_admin::ProposeCommand::UpdateElectedVersions {
                                                 release_artifact: update_version.release_artifact.clone(),
                                                 args: cli::Cli::get_update_cmd_args(&update_version)
                                             },
                                             ic_admin::ProposeOptions{
                                                 title: Some(update_version.title),
                                                 summary: Some(update_version.summary.clone()),
                                                 motivation: None,
                                             }, simulate)
                    }
                }
            },

            cli::Commands::Hostos(nodes) => {
                match &nodes.subcommand {
                    cli::hostos::Commands::Rollout { version,nodes} => {
                        let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                        runner.hostos_rollout(nodes.clone(), version, simulate, None).await
                    },
                    cli::hostos::Commands::RolloutFromNodeGroup {version, assignment, owner, nodes_in_group, exclude } => {
                        let update_group  = NodeGroupUpdate::new(*assignment, *owner, NumberOfNodes::from_str(nodes_in_group)?);
                        let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                        if let Some((nodes_to_update, summary)) = runner.hostos_rollout_nodes(update_group, version, exclude).await? {
                            return runner.hostos_rollout(nodes_to_update, version, simulate, Some(summary)).await
                        }
                        Ok(())
                    }
                }
            },
            cli::Commands::Nodes(nodes) => {
                match &nodes.subcommand {
                    cli::nodes::Commands::Remove { extra_nodes_filter, no_auto, remove_degraded, exclude, motivation } => {
                        if motivation.is_none() && !extra_nodes_filter.is_empty() {
                            cmd.error(
                                ErrorKind::MissingRequiredArgument,
                                "Required argument `motivation` not found",
                            )
                            .exit();
                        }
                        let runner = runner::Runner::new_with_network_url(cli::Cli::from_opts(&cli_opts, true).await?.into(), backend_port).await?;
                        runner.remove_nodes(NodesRemoveRequest {
                            extra_nodes_filter: extra_nodes_filter.clone(),
                            no_auto: *no_auto,
                            remove_degraded: *remove_degraded,
                            exclude: Some(exclude.clone()),
                            motivation: motivation.clone().unwrap_or_default(),
                        }, simulate).await
                    },
                }
            },

            cli::Commands::Vote {accepted_neurons, accepted_topics}=> {
                let cli = cli::Cli::from_opts(&cli_opts, true).await?;
                vote_on_proposals(match cli.get_neuron() {
                    Some(neuron) => neuron,
                    None => return Err(anyhow::anyhow!("Neuron required for this command")),
                }, cli.get_nns_url(), accepted_neurons, accepted_topics, simulate).await
            },

            cli::Commands::TrustworthyMetrics { wallet, start_at_timestamp, subnet_ids } => {
                let cli = cli::Cli::from_opts(&cli_opts, true).await?;
                get_node_metrics_history(CanisterId::from_str(wallet)?, subnet_ids.clone(), *start_at_timestamp, match cli.get_neuron() {
                    Some(neuron) => neuron,
                    None => return Err(anyhow::anyhow!("Neuron required for this command")),
                }, cli.get_nns_url()).await
            },

            cli::Commands::DumpRegistry { version, path } => {
                registry_dump::dump_registry(path, version, cli_opts.network).await
            }
        }
    })
    .await?;

    srv.stop(false).await;

    Ok(())
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

/// Get a localhost socket address with random, unused port.
fn local_unused_port() -> u16 {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )
    .unwrap();
    socket.bind(&addr.into()).unwrap();
    socket.set_reuse_address(true).unwrap();
    let tcp = std::net::TcpListener::from(socket);
    tcp.local_addr().unwrap().port()
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Default logging level is: info generally, warn for mio and actix_server
                // You can override defaults by setting environment variables
                // RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info,mio::=warn,actix_server::=warn")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
