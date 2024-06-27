use crate::ic_admin::IcAdminWrapper;
use clap::{error::ErrorKind, CommandFactory, Parser};
use decentralization::subnets::NodesRemover;
use dotenv::dotenv;
use dre::cli::proposals::ProposalStatus;
use dre::detect_neuron::Auth;
use dre::general::{filter_proposals, get_node_metrics_history, vote_on_proposals};
use dre::operations::hostos_rollout::{NodeGroupUpdate, NumberOfNodes};
use dre::{cli, ic_admin, registry_dump, runner};
use ic_base_types::CanisterId;
use ic_canisters::governance::{governance_canister_version, GovernanceCanisterWrapper};
use ic_canisters::CanisterClient;
use ic_management_types::filter_map_nns_function_proposals;
use ic_management_types::{Artifact, MinNakamotoCoefficients, NodeFeature};

use ic_nns_common::pb::v1::ProposalId;
use ic_nns_governance::pb::v1::ListProposalInfo;
use log::{info, warn};
use regex::Regex;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

const STAGING_NEURON_ID: u64 = 49;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logger();
    let version = env!("CARGO_PKG_VERSION");
    info!("Running version {}", version);

    dotenv().ok();

    let mut cmd = cli::Opts::command();
    let mut cli_opts = cli::Opts::parse();

    if let cli::Commands::Upgrade = &cli_opts.subcommand {
        let response = tokio::task::spawn_blocking(move || check_latest_release(version, true)).await??;
        match response {
            UpdateStatus::NoUpdate => info!("Running the latest version"),
            UpdateStatus::NewVersion(_) => unreachable!("Shouldn't happen"),
            UpdateStatus::Updated(v) => info!("Upgraded: {} -> {}", version, v),
        }
        return Ok(());
    }

    let handle = tokio::task::spawn_blocking(move || check_latest_release(version, false));

    let target_network = ic_management_types::Network::new(cli_opts.network.clone(), &cli_opts.nns_urls)
        .await
        .expect("Failed to create network");
    let nns_urls = target_network.get_nns_urls();

    // Start of actually doing stuff with commands.
    if target_network.name == "staging" {
        if cli_opts.private_key_pem.is_none() {
            cli_opts.private_key_pem =
                Some(std::env::var("HOME").expect("Please set HOME env var") + "/.config/dfx/identity/bootstrap-super-leader/identity.pem");
        }
        if cli_opts.neuron_id.is_none() {
            cli_opts.neuron_id = Some(STAGING_NEURON_ID);
        }
    }
    let governance_canister_v = match governance_canister_version(nns_urls).await {
        Ok(c) => c,
        Err(e) => return Err(anyhow::anyhow!("While determining the governance canister version: {}", e)),
    };

    let governance_canister_version = governance_canister_v.stringified_hash;

    let r = ic_admin::with_ic_admin(governance_canister_version.into(), async {
        let dry_run = cli_opts.dry_run;
        let cli = dre::parsed_cli::ParsedCli::from_opts(&cli_opts)
            .await
            .expect("Failed to create authenticated CLI");
        let ic_admin_wrapper = IcAdminWrapper::from_cli(cli);

        let runner_instance = runner::Runner::new(ic_admin_wrapper, &target_network)
            .await
            .expect("Failed to create a runner");

        let r = match &cli_opts.subcommand {
            // Covered above
            cli::Commands::Upgrade => Ok(()),
            cli::Commands::DerToPrincipal { path } => {
                let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
                println!("{}", principal);
                Ok(())
            }

            cli::Commands::Heal {
                max_replaceable_nodes_per_sub,
            } => {
                runner_instance
                    .network_heal(
                        *max_replaceable_nodes_per_sub,
                        cli_opts.verbose,
                        dry_run,
                    )
                    .await
            }

            cli::Commands::Subnet(subnet) => {
                // Check if required arguments are provided
                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { .. } | cli::subnet::Commands::Resize { .. } | cli::subnet::Commands::Rescue { .. } => {
                        if subnet.id.is_none() {
                            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `id` not found").exit();
                        }
                    }
                    cli::subnet::Commands::Replace { nodes, .. } => {
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

                // Execute the command
                match &subnet.subcommand {
                    cli::subnet::Commands::Deploy { version } => runner_instance.deploy(&subnet.id.unwrap(), version, dry_run).await,
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
                        runner_instance
                            .membership_replace(
                                ic_management_types::requests::MembershipReplaceRequest {
                                    target: match &subnet.id {
                                        Some(subnet) => ic_management_types::requests::ReplaceTarget::Subnet(*subnet),
                                        None => {
                                            if let Some(motivation) = motivation.clone() {
                                                ic_management_types::requests::ReplaceTarget::Nodes {
                                                    nodes: nodes.clone(),
                                                    motivation,
                                                }
                                            } else {
                                                cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `motivation` not found")
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
                                },
                                cli_opts.verbose,
                                dry_run,
                            )
                            .await
                    }
                    cli::subnet::Commands::Resize {
                        add,
                        remove,
                        include,
                        only,
                        exclude,
                        motivation,
                    } => {
                        if let Some(motivation) = motivation.clone() {
                            runner_instance
                                .subnet_resize(
                                    ic_management_types::requests::SubnetResizeRequest {
                                        subnet: subnet.id.unwrap(),
                                        add: *add,
                                        remove: *remove,
                                        only: only.clone().into(),
                                        exclude: exclude.clone().into(),
                                        include: include.clone().into(),
                                    },
                                    motivation,
                                    cli_opts.verbose,
                                    dry_run,
                                )
                                .await
                        } else {
                            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `motivation` not found")
                                .exit();
                        }
                    }
                    cli::subnet::Commands::Create {
                        size,
                        min_nakamoto_coefficients,
                        exclude,
                        only,
                        include,
                        motivation,
                        replica_version,
                        other_args,
                        help_other_args,
                    } => {
                        let min_nakamoto_coefficients = parse_min_nakamoto_coefficients(&mut cmd, min_nakamoto_coefficients);
                        let motivation = if motivation.is_none() && *help_other_args {
                            Some("help for options".to_string())
                        } else {
                            motivation.clone()
                        };
                        if let Some(motivation) = motivation {
                            runner_instance
                                .subnet_create(
                                    ic_management_types::requests::SubnetCreateRequest {
                                        size: *size,
                                        min_nakamoto_coefficients,
                                        only: only.clone().into(),
                                        exclude: exclude.clone().into(),
                                        include: include.clone().into(),
                                    },
                                    motivation,
                                    cli_opts.verbose,
                                    dry_run,
                                    replica_version.clone(),
                                    other_args.to_vec(),
                                    *help_other_args,
                                )
                                .await
                        } else {
                            cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `motivation` not found")
                                .exit();
                        }
                    }
                    cli::subnet::Commands::Rescue { keep_nodes } => {
                        runner_instance.subnet_rescue(&subnet.id.unwrap(), keep_nodes.clone(), dry_run).await
                    }
                }
            }

            cli::Commands::Get { args } => {
                runner_instance.ic_admin.run_passthrough_get(args, false).await?;
                Ok(())
            }

            cli::Commands::Propose { args } => runner_instance.ic_admin.run_passthrough_propose(args, dry_run).await,

            cli::Commands::UpdateUnassignedNodes { nns_subnet_id } => {
                let ic_admin = if target_network.is_mainnet() {
                    runner_instance.ic_admin.clone().as_automation()
                } else {
                    runner_instance.ic_admin.clone()
                };
                let nns_subnet_id = match nns_subnet_id {
                    Some(subnet_id) => subnet_id.to_owned(),
                    None => {
                        let res = ic_admin.run_passthrough_get(&["get-subnet-list".to_string()], true).await?;
                        let subnet_list: Vec<String> = serde_json::from_str(&res)?;
                        subnet_list.first().ok_or_else(|| anyhow::anyhow!("No subnet found"))?.clone()
                    }
                };
                ic_admin.update_unassigned_nodes(&nns_subnet_id, &target_network, dry_run).await
            }

            cli::Commands::Version(version_command) => match &version_command {
                cli::version::Cmd::ReviseElectedVersions(update_command) => {
                    let release_artifact: &Artifact = &update_command.subcommand.clone().into();

                    let update_version = match &update_command.subcommand {
                        cli::version::ReviseElectedVersionsCommands::GuestOS { version, release_tag, force }
                        | cli::version::ReviseElectedVersionsCommands::HostOS { version, release_tag, force } => {
                            ic_admin::IcAdminWrapper::prepare_to_propose_to_revise_elected_versions(
                                release_artifact,
                                version,
                                release_tag,
                                *force,
                                runner_instance
                                    .prepare_versions_to_retire(release_artifact, false)
                                    .await
                                    .map(|res| res.1)?,
                            )
                        }
                    }
                    .await?;

                    runner_instance
                        .ic_admin
                        .propose_run(
                            ic_admin::ProposeCommand::ReviseElectedVersions {
                                release_artifact: update_version.release_artifact.clone(),
                                args: dre::parsed_cli::ParsedCli::get_update_cmd_args(&update_version),
                            },
                            ic_admin::ProposeOptions {
                                title: Some(update_version.title),
                                summary: Some(update_version.summary.clone()),
                                motivation: None,
                            },
                            dry_run,
                        )
                        .await?;
                    Ok(())
                }
            },

            cli::Commands::Hostos(nodes) => {
                let as_automation = target_network.is_mainnet();
                match &nodes.subcommand {
                    cli::hostos::Commands::Rollout { version, nodes } => {
                        runner_instance.hostos_rollout(nodes.clone(), version, dry_run, None, as_automation).await
                    }
                    cli::hostos::Commands::RolloutFromNodeGroup {
                        version,
                        assignment,
                        owner,
                        nodes_in_group,
                        exclude,
                    } => {
                        let update_group = NodeGroupUpdate::new(*assignment, *owner, NumberOfNodes::from_str(nodes_in_group)?);
                        if let Some((nodes_to_update, summary)) = runner_instance.hostos_rollout_nodes(update_group, version, exclude).await? {
                            return runner_instance
                                .hostos_rollout(nodes_to_update, version, dry_run, Some(summary), as_automation)
                                .await;
                        }
                        Ok(())
                    }
                }
            }

            cli::Commands::Nodes(nodes) => match &nodes.subcommand {
                cli::nodes::Commands::Remove {
                    extra_nodes_filter,
                    no_auto,
                    remove_degraded,
                    exclude,
                    motivation,
                } => {
                    if motivation.is_none() && !extra_nodes_filter.is_empty() {
                        cmd.error(ErrorKind::MissingRequiredArgument, "Required argument `motivation` not found")
                            .exit();
                    }
                    runner_instance
                        .remove_nodes(
                            NodesRemover {
                                extra_nodes_filter: extra_nodes_filter.clone(),
                                no_auto: *no_auto,
                                remove_degraded: *remove_degraded,
                                exclude: Some(exclude.clone()),
                                motivation: motivation.clone().unwrap_or_default(),
                            },
                            dry_run,
                        )
                        .await
                }
            },

            cli::Commands::ApiBoundaryNodes(api_boundary_nodes) => match &api_boundary_nodes.subcommand {
                cli::api_boundary_nodes::Commands::Update { nodes, version, motivation } => {
                    runner_instance
                        .ic_admin
                        .propose_run(
                            ic_admin::ProposeCommand::DeployGuestosToSomeApiBoundaryNodes {
                                nodes: nodes.to_vec(),
                                version: version.to_string(),
                            },
                            ic_admin::ProposeOptions {
                                title: Some(format!("Update {} API boundary node(s) to {version}", nodes.clone().len())),
                                summary: Some(format!("Update {} API boundary node(s) to {version}", nodes.clone().len())),
                                motivation: motivation.clone(),
                            },
                            dry_run,
                        )
                        .await?;
                    Ok(())
                }
                cli::api_boundary_nodes::Commands::Add { nodes, version, motivation } => {
                    runner_instance
                        .ic_admin
                        .propose_run(
                            ic_admin::ProposeCommand::AddApiBoundaryNodes {
                                nodes: nodes.to_vec(),
                                version: version.to_string(),
                            },
                            ic_admin::ProposeOptions {
                                title: Some(format!("Add {} API boundary node(s)", nodes.clone().len())),
                                summary: Some(format!("Add {} API boundary node(s)", nodes.clone().len())),
                                motivation: motivation.clone(),
                            },
                            dry_run,
                        )
                        .await?;
                    Ok(())
                }
                cli::api_boundary_nodes::Commands::Remove { nodes, motivation } => {
                    runner_instance
                        .ic_admin
                        .propose_run(
                            ic_admin::ProposeCommand::RemoveApiBoundaryNodes { nodes: nodes.to_vec() },
                            ic_admin::ProposeOptions {
                                title: Some(format!("Remove {} API boundary node(s)", nodes.clone().len())),
                                summary: Some(format!("Remove {} API boundary node(s)", nodes.clone().len())),
                                motivation: motivation.clone(),
                            },
                            dry_run,
                        )
                        .await?;
                    Ok(())
                }
            },

            cli::Commands::Vote {
                accepted_neurons,
                accepted_topics,
                sleep_time,
            } => {
                let cli = dre::parsed_cli::ParsedCli::from_opts(&cli_opts).await?;
                vote_on_proposals(
                    cli.get_neuron(),
                    target_network.get_nns_urls(),
                    accepted_neurons,
                    accepted_topics,
                    dry_run,
                    *sleep_time,
                )
                .await
            }

            cli::Commands::TrustworthyMetrics {
                wallet,
                start_at_timestamp,
                subnet_ids,
            } => {
                let auth = Auth::from_cli_args(cli_opts.private_key_pem, cli_opts.hsm_slot, cli_opts.hsm_pin, cli_opts.hsm_key_id)?;
                get_node_metrics_history(
                    CanisterId::from_str(wallet)?,
                    subnet_ids.clone(),
                    *start_at_timestamp,
                    &auth,
                    target_network.get_nns_urls(),
                )
                .await
            }

            cli::Commands::Registry {
                version,
                output,
                local_registry_path,
                incorrect_rewards,
            } => registry_dump::dump_registry(local_registry_path, &target_network, version, output, *incorrect_rewards).await,

            cli::Commands::Firewall { title, summary, rules_scope } => {
                runner_instance
                    .ic_admin
                    .update_firewall(
                        &target_network,
                        ic_admin::ProposeOptions {
                            title: title.clone(),
                            summary: summary.clone(),
                            ..Default::default()
                        },
                        rules_scope,
                        cli_opts.dry_run,
                    )
                    .await
            }
            cli::Commands::Proposals(p) => match &p.subcommand {
                cli::proposals::Commands::Pending => {
                    let nns_url = target_network.get_nns_urls().first().expect("Should have at least one NNS URL");
                    let client = GovernanceCanisterWrapper::from(CanisterClient::from_anonymous(nns_url)?);
                    let proposals = client.get_pending_proposals().await?;
                    let proposals = serde_json::to_string(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;
                    println!("{}", proposals);
                    Ok(())
                }
                cli::proposals::Commands::List {
                    limit,
                    before_proposal,
                    exclude_topic,
                    include_reward_status,
                    include_status,
                    include_all_manage_neuron_proposals,
                    omit_large_fields,
                } => {
                    let nns_url = target_network.get_nns_urls().first().expect("Should have at least one NNS URL");
                    let client = GovernanceCanisterWrapper::from(CanisterClient::from_anonymous(nns_url)?);
                    let proposals = client
                        .list_proposals(ListProposalInfo {
                            before_proposal: before_proposal.as_ref().map(|p| ProposalId { id: *p }),
                            exclude_topic: exclude_topic.clone(),
                            include_all_manage_neuron_proposals: *include_all_manage_neuron_proposals,
                            include_reward_status: include_reward_status.clone(),
                            include_status: include_status.clone(),
                            limit: *limit,
                            omit_large_fields: *omit_large_fields,
                        })
                        .await?
                        .into_iter()
                        .map(|p| {
                            dre::general::Proposal::try_from(p.clone())
                                .map(|r| serde_json::to_value(r).expect("cannot serialize to json"))
                                .unwrap_or_else(|_e| serde_json::to_value(p).expect("cannot serialize to json"))
                        })
                        .collect::<Vec<_>>();
                    let proposals = serde_json::to_string_pretty(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;
                    println!("{}", proposals);
                    Ok(())
                }
                cli::proposals::Commands::Filter { limit, statuses, topics } => {
                    filter_proposals(
                        target_network,
                        limit,
                        statuses.iter().map(|s| s.clone().into()).collect(),
                        topics.iter().map(|t| t.clone().into()).collect(),
                    )
                    .await
                }
                cli::proposals::Commands::Get { proposal_id } => {
                    let nns_url = target_network.get_nns_urls().first().expect("Should have at least one NNS URL");
                    let client = GovernanceCanisterWrapper::from(CanisterClient::from_anonymous(nns_url)?);
                    let proposal = client.get_proposal(*proposal_id).await?;
                    let proposal = serde_json::to_string_pretty(&proposal).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;
                    println!("{}", proposal);
                    Ok(())
                }
                cli::proposals::Commands::Analyze { proposal_id } => {
                    let nns_url = target_network.get_nns_urls().first().expect("Should have at least one NNS URL");
                    let client = GovernanceCanisterWrapper::from(CanisterClient::from_anonymous(nns_url)?);
                    let proposal = client.get_proposal(*proposal_id).await?;

                    return if proposal.status() == ProposalStatus::Open.into() {
                        if let Some((_, change_membership)) =
                            filter_map_nns_function_proposals::<ChangeSubnetMembershipPayload>(&vec![proposal]).first()
                        {
                            runner_instance.decentralization_change(change_membership).await
                        } else {
                            Err(anyhow::anyhow!(
                                "Proposal {} must have {} type",
                                proposal_id,
                                ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetMembership.as_str_name()
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Proposal {} has status {}\nProposal must have status: {}",
                            proposal_id,
                            proposal.status().as_str_name(),
                            ProposalStatus::Open
                        ))
                    };
                }
            },
        };
        let _ = runner_instance.stop_backend().await;
        r
    })
    .await;

    let maybe_update_status = handle.await?;
    match maybe_update_status {
        Ok(s) => match s {
            UpdateStatus::NoUpdate => {}
            UpdateStatus::NewVersion(v) => info!("There is a new version '{}' available. Run 'dre upgrade' to upgrade", v),
            UpdateStatus::Updated(_) => unreachable!("Shouldn't happen"),
        },
        Err(e) => warn!("There was an error while checking for new updates: {:?}", e),
    }

    r
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
fn parse_min_nakamoto_coefficients(cmd: &mut clap::Command, min_nakamoto_coefficients: &[String]) -> Option<MinNakamotoCoefficients> {
    let min_nakamoto_coefficients: Vec<String> = if min_nakamoto_coefficients.is_empty() {
        ["node_provider=5", "average=3"].iter().map(|s| String::from(*s)).collect()
    } else {
        min_nakamoto_coefficients.to_vec()
    };

    let mut average = 3.0;
    let min_nakamoto_coefficients = min_nakamoto_coefficients
        .iter()
        .filter_map(|s| {
            let (key, val) = match s.split_once('=') {
                Some(s) => s,
                None => cmd.error(ErrorKind::ValueValidation, "Value requires exactly one '=' symbol").exit(),
            };
            if key.to_lowercase() == "average" {
                average = val
                    .parse::<f64>()
                    .map_err(|_| cmd.error(ErrorKind::ValueValidation, "Failed to parse feature from string").exit())
                    .unwrap();
                None
            } else {
                let feature = match NodeFeature::from_str(key) {
                    Ok(v) => v,
                    Err(_) => cmd.error(ErrorKind::ValueValidation, "Failed to parse feature from string").exit(),
                };
                let val: f64 = val
                    .parse::<f64>()
                    .map_err(|_| cmd.error(ErrorKind::ValueValidation, "Failed to parse feature from string").exit())
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
                // Default logging level is: info generally, warn for mio and actix_server
                // You can override defaults by setting environment variables
                // RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info,mio::=warn,actix_server::=warn")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}

fn check_latest_release(curr_version: &str, proceed_with_upgrade: bool) -> anyhow::Result<UpdateStatus> {
    // ^                --> start of line
    // v?               --> optional 'v' char
    // (\d+\.\d+\.\d+)  --> string in format '1.22.33'
    // (-([0-9a-f])+)   --> string in format '-12345af' (optional)
    let re_version = Regex::new(r"^v?(\d+\.\d+\.\d+)(-([0-9a-f])+(\-dirty)?)?$").unwrap();
    let current_version = match re_version.captures(curr_version) {
        Some(cap) => cap.get(1).unwrap().as_str(),
        None => return Err(anyhow::anyhow!("Version '{}' doesn't follow expected naming", curr_version)),
    };

    let maybe_configured_backend = self_update::backends::github::ReleaseList::configure()
        .repo_owner("dfinity")
        .repo_name("dre")
        .build()
        .map_err(|e| anyhow::anyhow!("Configuring backend failed: {:?}", e))?;

    let releases = maybe_configured_backend
        .fetch()
        .map_err(|e| anyhow::anyhow!("Fetching releases failed: {:?}", e))?;

    let latest_release = match releases.first() {
        Some(v) => v,
        None => return Err(anyhow::anyhow!("No releases found")),
    };

    if latest_release.version.eq(current_version) {
        return Ok(UpdateStatus::NoUpdate);
    }

    if !proceed_with_upgrade {
        return Ok(UpdateStatus::NewVersion(latest_release.version.clone()));
    }

    info!("Binary not up to date. Updating to {}", latest_release.version);

    let asset = match latest_release.asset_for("dre", None) {
        Some(asset) => asset,
        None => return Err(anyhow::anyhow!("No assets found for release")),
    };

    let tmp_dir = tempfile::Builder::new()
        .prefix("self_update")
        .tempdir_in(::std::env::current_dir().unwrap())
        .map_err(|e| anyhow::anyhow!("Couldn't create temp dir: {:?}", e))?;

    let new_dre_path = tmp_dir.path().join(&asset.name);
    let asset_path = tmp_dir.path().join("asset");
    let asset_file = std::fs::File::create(&asset_path).map_err(|e| anyhow::anyhow!("Couldn't create file: {:?}", e))?;
    let new_dre_file = std::fs::File::create(&new_dre_path).map_err(|e| anyhow::anyhow!("Couldn't create file: {:?}", e))?;

    self_update::Download::from_url(&asset.download_url)
        .show_progress(true)
        .download_to(&asset_file)
        .map_err(|e| anyhow::anyhow!("Couldn't download asset: {:?}", e))?;

    info!("Asset downloaded successfully");

    let value: Value =
        serde_json::from_str(&std::fs::read_to_string(&asset_path).unwrap()).map_err(|e| anyhow::anyhow!("Couldn't open asset: {:?}", e))?;

    let download_url = match value.get("browser_download_url") {
        Some(Value::String(d)) => d,
        Some(_) => return Err(anyhow::anyhow!("Unexpected type for url in asset")),
        None => return Err(anyhow::anyhow!("Download url not present in asset")),
    };

    self_update::Download::from_url(download_url)
        .show_progress(true)
        .download_to(&new_dre_file)
        .map_err(|e| anyhow::anyhow!("Couldn't download binary: {:?}", e))?;

    self_update::self_replace::self_replace(new_dre_path).map_err(|e| anyhow::anyhow!("Couldn't upgrade to the newest version: {:?}", e))?;

    Ok(UpdateStatus::Updated(latest_release.version.clone()))
}

enum UpdateStatus {
    NoUpdate,
    NewVersion(String),
    Updated(String),
}
