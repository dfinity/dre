use crate::cli::version::Commands::{Bless, Retire};
use clap::{CommandFactory, ErrorKind, Parser};
use clients::DashboardBackendClient;
use ic_management_types::{MinNakamotoCoefficients, NodeFeature};
use std::collections::BTreeMap;
use std::str::FromStr;

mod cli;
mod clients;
pub(crate) mod defaults;
mod ic_admin;
mod ops_subnet_node_replace;
mod runner;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logger();
    let cli_opts = cli::Opts::parse();
    let mut cmd = cli::Opts::command();

    ic_admin::with_ic_admin(Default::default(), async {
        // Start of actually doing stuff with commands.
        match &cli_opts.subcommand {
            cli::Commands::DerToPrincipal { path } => {
                let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
                println!("{}", principal);
                Ok(())
            }

            cli::Commands::Subnet(subnet) => {
                let runner = runner::Runner::from_opts(&cli_opts).await?;
                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { .. } | cli::subnet::Commands::Extend { .. } => {
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
                }

                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { version } => runner.deploy(&subnet.id.unwrap(), version),
                    cli::subnet::Commands::Replace {
                        nodes,
                        no_heal,
                        optimize,
                        motivation,
                        exclude,
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
                                    include: include.clone().into(),
                                    min_nakamoto_coefficients,
                                }, *verbose)
                                .await
                    }
                    cli::subnet::Commands::Extend { size, include, exclude, motivation, verbose, } => {
                        if let Some(motivation) = motivation.clone() {
                            runner.subnet_extend(ic_management_types::requests::SubnetExtendRequest {
                                subnet: subnet.id.unwrap(),
                                size: *size,
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
                    Bless { version } => {
                        let ic_admin = ic_admin::Cli::from_opts(&cli_opts, true).await?;
                        let (summary, cmd) = ic_admin::Cli::prepare_to_propose_to_bless_new_replica_version(version).await?;
                        ic_admin.propose_run(cmd, ic_admin::ProposeOptions { title: Some(format!("Elect new replica binary revision (commit {version})")), summary: Some(summary), motivation: None })
                    },
                    Retire { edit_summary } => {
                        let ic_admin = ic_admin::Cli::from_opts(&cli_opts, true).await?;
                        let dashboard_client = DashboardBackendClient::new(cli_opts.network, cli_opts.dev);
                        let (summary, cmd ) = ic_admin.get_replica_versions_to_retire(*edit_summary, dashboard_client).await?;

                        ic_admin.propose_run(cmd, ic_admin::ProposeOptions { title: Some("Retire IC replica version".to_string()), summary: Some(summary), motivation: None })
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
