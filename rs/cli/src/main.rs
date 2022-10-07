use clap::{CommandFactory, ErrorKind, Parser};
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
                    cli::subnet::Commands::Deploy { .. } => {
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
                    } => {
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
                                })
                                .await
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
        }
    })
    .await
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
