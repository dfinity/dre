use crate::auth::Neuron;
use anyhow::{anyhow, Result};
use colored::Colorize;
use dialoguer::Confirm;
use flate2::read::GzDecoder;
use futures::future::BoxFuture;
use ic_base_types::PrincipalId;
use ic_management_types::{Artifact, Network};
use log::{error, info};
use mockall::automock;
use regex::Regex;
use shlex::try_quote;
use std::fmt::Debug;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::{path::Path, process::Command};
use strum::Display;

const MAX_SUMMARY_CHAR_COUNT: usize = 29000;

#[automock]
pub trait IcAdmin: Send + Sync + Debug {
    fn ic_admin_path(&self) -> Option<String>;

    fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions) -> BoxFuture<'_, anyhow::Result<String>>;

    fn run<'a>(&'a self, command: &'a str, args: &'a [String], silent: bool) -> BoxFuture<'_, anyhow::Result<String>>;

    fn grep_subcommand_arguments(&self, subcommand: &str) -> String;

    fn run_passthrough_get<'a>(&'a self, args: &'a [String], silent: bool) -> BoxFuture<'_, anyhow::Result<String>>;

    fn run_passthrough_propose<'a>(&'a self, args: &'a [String]) -> BoxFuture<'_, anyhow::Result<String>>;
}

#[derive(Clone, Debug)]
pub struct IcAdminImpl {
    network: Network,
    ic_admin_bin_path: Option<String>,
    proceed_without_confirmation: bool,
    neuron: Neuron,
    dry_run: bool,
}

impl IcAdmin for IcAdminImpl {
    fn ic_admin_path(&self) -> Option<String> {
        self.ic_admin_bin_path.clone()
    }

    fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions) -> BoxFuture<'_, anyhow::Result<String>> {
        Box::pin(async move { self.propose_run_inner(cmd, opts, self.dry_run).await })
    }

    fn run<'a>(&'a self, command: &'a str, args: &'a [String], silent: bool) -> BoxFuture<'_, anyhow::Result<String>> {
        let ic_admin_args = [&[command.to_string()], args].concat();
        Box::pin(async move { self._run_ic_admin_with_args(&ic_admin_args, silent).await })
    }

    fn grep_subcommand_arguments(&self, subcommand: &str) -> String {
        let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
        let cmd_result = Command::new(ic_admin_path).args([subcommand, "--help"]).output();
        match cmd_result.map_err(|e| e.to_string()) {
            Ok(output) => {
                if output.status.success() {
                    String::from_utf8_lossy(output.stdout.as_ref()).to_string()
                } else {
                    error!("Execution of ic-admin failed: {}", String::from_utf8_lossy(output.stderr.as_ref()));
                    String::new()
                }
            }
            Err(err) => {
                error!("Error starting ic-admin process: {}", err);
                String::new()
            }
        }
    }

    /// Run an `ic-admin get-*` command directly, and without an HSM
    fn run_passthrough_get<'a>(&'a self, args: &'a [String], silent: bool) -> BoxFuture<'_, anyhow::Result<String>> {
        if args.is_empty() {
            println!("List of available ic-admin 'get' sub-commands:\n");
            for subcmd in self.grep_subcommands(r"\s+get-(.+?)\s") {
                println!("\t{}", subcmd)
            }
            std::process::exit(1);
        }

        // The `get` subcommand of the cli expects that "get-" prefix is not provided as
        // the ic-admin command
        let args = if args[0].starts_with("get-") {
            // The user did provide the "get-" prefix, so let's just keep it and use it.
            // This provides a convenient backward compatibility with ic-admin commands
            // i.e., `dre get get-subnet 0` still works, although `dre get
            // subnet 0` is preferred
            args.to_vec()
        } else {
            // But since ic-admin expects these commands to include the "get-" prefix, we
            // need to add it back Example:
            // `dre get subnet 0` becomes
            // `ic-admin --nns-url "http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080" get-subnet 0`
            let mut args_with_get_prefix = vec![String::from("get-") + args[0].as_str()];
            args_with_get_prefix.extend_from_slice(args.split_at(1).1);
            args_with_get_prefix
        };

        Box::pin(async move { self.run(&args[0], &args.iter().skip(1).cloned().collect::<Vec<_>>(), silent).await })
    }

    /// Run an `ic-admin propose-to-*` command directly
    fn run_passthrough_propose<'a>(&'a self, args: &'a [String]) -> BoxFuture<'_, anyhow::Result<String>> {
        if args.is_empty() {
            println!("List of available ic-admin 'propose' sub-commands:\n");
            for subcmd in self.grep_subcommands(r"\s+propose-to-(.+?)\s") {
                println!("\t{}", subcmd)
            }
            std::process::exit(1);
        }

        // The `propose` subcommand of the cli expects that "propose-to-" prefix is not
        // provided as the ic-admin command
        let args = if args[0].starts_with("propose-to-") {
            // The user did provide the "propose-to-" prefix, so let's just keep it and use
            // it.
            args.to_vec()
        } else {
            // But since ic-admin expects these commands to include the "propose-to-"
            // prefix, we need to add it back.
            let mut args_with_fixed_prefix = vec![String::from("propose-to-") + args[0].as_str()];
            args_with_fixed_prefix.extend_from_slice(args.split_at(1).1);
            args_with_fixed_prefix
        };

        // ic-admin expects --summary and not --motivation
        // make sure the expected argument is provided
        let args = if !args.contains(&String::from("--summary")) && args.contains(&String::from("--motivation")) {
            args.iter()
                .map(|arg| if arg == "--motivation" { "--summary".to_string() } else { arg.clone() })
                .collect::<Vec<_>>()
        } else {
            args.to_vec()
        };

        let cmd = ProposeCommand::Raw {
            command: args[0].clone(),
            args: args.iter().skip(1).cloned().collect::<Vec<_>>(),
        };
        let dry_run = self.dry_run || cmd.args().contains(&String::from("--dry-run"));
        Box::pin(async move { self.propose_run_inner(cmd, Default::default(), dry_run).await })
    }
}

impl IcAdminImpl {
    pub fn new(network: Network, ic_admin_bin_path: Option<String>, proceed_without_confirmation: bool, neuron: Neuron, dry_run: bool) -> Self {
        Self {
            network,
            ic_admin_bin_path,
            proceed_without_confirmation,
            neuron,
            dry_run,
        }
    }

    async fn print_ic_admin_command_line(&self, cmd: &Command) {
        info!(
            "running ic-admin: \n$ {}{}",
            cmd.get_program().to_str().unwrap().yellow(),
            cmd.get_args()
                .map(|s| s.to_str().unwrap().to_string())
                .fold("".to_string(), |acc, s| {
                    // If previous argument was pin it means that this one is the pin itself
                    if acc.ends_with("--pin") {
                        format!("{acc} <redacted>")
                    } else {
                        let s = if s.contains('\n') {
                            format!("'{}'", s.replace('\'', "'\\''"))
                        } else {
                            match try_quote(s.as_str()) {
                                Ok(sss) => sss.to_string(),
                                Err(_e) => s,
                            }
                        };
                        if s.starts_with("--") {
                            format!("{acc} \\\n    {s}")
                        } else if !acc.split(' ').last().unwrap_or_default().starts_with("--") {
                            format!("{acc} \\\n  {s}")
                        } else {
                            format!("{acc} {s}")
                        }
                    }
                })
                .yellow(),
        );
    }

    async fn _exec(
        &self,
        cmd: ProposeCommand,
        opts: ProposeOptions,
        as_simulation: bool,
        print_out_command: bool,
        print_ic_admin_out: bool,
    ) -> anyhow::Result<String> {
        if let Some(summary) = opts.clone().summary {
            let summary_count = summary.chars().count();
            if summary_count > MAX_SUMMARY_CHAR_COUNT {
                return Err(anyhow!(
                    "Summary length {} exceeded MAX_SUMMARY_CHAR_COUNT {}",
                    summary_count,
                    MAX_SUMMARY_CHAR_COUNT,
                ));
            }
        }

        let cmd_out = self
            .run(
                &cmd.get_command_name(),
                [
                    // Make sure there is no more than one `--dry-run` argument, or else ic-admin will complain.
                    if as_simulation && !cmd.args().contains(&String::from("--dry-run")) {
                        vec!["--dry-run".to_string()]
                    } else {
                        Default::default()
                    },
                    opts.title.map(|t| vec!["--proposal-title".to_string(), t]).unwrap_or_default(),
                    opts.summary
                        .map(|s| {
                            vec![
                                "--summary".to_string(),
                                format!(
                                    "{}{}",
                                    s,
                                    opts.motivation
                                        .map(|m| format!(
                                            "\n\nMotivation: {m}{}",
                                            match opts.forum_post_link {
                                                Some(link) => format!("\nForum post link: {}\n", link),
                                                None => "".to_string(),
                                            }
                                        ))
                                        .unwrap_or_default(),
                                ),
                            ]
                        })
                        .unwrap_or_default(),
                    cmd.args(),
                    self.neuron.proposer_as_arg_vec(),
                ]
                .concat()
                .as_slice(),
                print_out_command,
            )
            .await?;

        if print_ic_admin_out {
            println!("{}", cmd_out)
        }

        Ok(cmd_out)
    }

    async fn propose_run_inner(&self, cmd: ProposeCommand, opts: ProposeOptions, dry_run: bool) -> anyhow::Result<String> {
        let opts = if opts.forum_post_link.is_some() || self.proceed_without_confirmation {
            opts
        } else {
            println!(
                "Proposal title: {}",
                opts.title.as_deref().unwrap_or(opts.summary.as_deref().unwrap_or(""))
            );
            let forum_post_link = Confirm::new()
                .with_prompt("Link to a forum thread not found. Do you want to add it?")
                .default(true)
                .interact()?;
            if forum_post_link {
                let forum_post_link = dialoguer::Input::<String>::new()
                    .with_prompt("Forum post link")
                    .allow_empty(true)
                    .interact()?;
                ProposeOptions {
                    forum_post_link: Some(forum_post_link),
                    ..opts
                }
            } else {
                opts
            }
        };

        // Dry run, or --help executions run immediately and do not proceed.
        if dry_run || cmd.args().contains(&String::from("--help")) || cmd.args().contains(&String::from("--dry-run")) {
            return self._exec(cmd, opts, true, false, false).await;
        }

        // If --yes was specified, don't ask the user if they want to proceed
        if !self.proceed_without_confirmation {
            self._exec(cmd.clone(), opts.clone(), true, false, true).await?;
        }

        if self.proceed_without_confirmation || Confirm::new().with_prompt("Do you want to continue?").default(false).interact()? {
            // User confirmed the desire to submit the proposal and no obvious problems were
            // found. Proceeding!
            self._exec(cmd, opts, false, true, true).await
        } else {
            Err(anyhow::anyhow!("Action aborted"))
        }
    }

    async fn _run_ic_admin_with_args(&self, ic_admin_args: &[String], silent: bool) -> anyhow::Result<String> {
        let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let auth_options = self.neuron.as_arg_vec();
        let root_options = [auth_options, vec!["--nns-urls".to_string(), self.network.get_nns_urls_string()]].concat();
        let cmd = cmd.args([&root_options, ic_admin_args].concat());

        if silent {
            cmd.stderr(Stdio::piped());
        } else {
            self.print_ic_admin_command_line(cmd).await;
        }
        cmd.stdout(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => match child.wait() {
                Ok(s) => {
                    if s.success() {
                        if let Some(mut output) = child.stdout {
                            let mut readbuf = vec![];
                            output
                                .read_to_end(&mut readbuf)
                                .map_err(|e| anyhow::anyhow!("Error reading output: {:?}", e))?;
                            let converted = String::from_utf8_lossy(&readbuf).trim().to_string();
                            if !silent {
                                println!("{}", converted);
                            }
                            return Ok(converted);
                        }
                        Ok("".to_string())
                    } else {
                        let readbuf = match child.stderr {
                            Some(mut stderr) => {
                                let mut readbuf = String::new();
                                stderr
                                    .read_to_string(&mut readbuf)
                                    .map_err(|e| anyhow::anyhow!("Error reading output: {:?}", e))?;
                                readbuf
                            }
                            None => "".to_string(),
                        };
                        Err(anyhow::anyhow!(
                            "ic-admin failed with non-zero exit code {} stderr ==>\n{}",
                            s.code().map(|c| c.to_string()).unwrap_or_else(|| "<none>".to_string()),
                            readbuf
                        ))
                    }
                }
                Err(err) => Err(anyhow::format_err!("ic-admin wasn't running: {}", err.to_string())),
            },
            Err(e) => Err(anyhow::format_err!("failed to run ic-admin: {}", e.to_string())),
        }
    }

    /// Run ic-admin and parse sub-commands that it lists with "--help",
    /// extract the ones matching `needle_regex` and return them as a
    /// `Vec<String>`
    fn grep_subcommands(&self, needle_regex: &str) -> Vec<String> {
        let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
        let cmd_result = Command::new(ic_admin_path).args(["--help"]).output();
        match cmd_result.map_err(|e| e.to_string()) {
            Ok(output) => {
                if output.status.success() {
                    let cmd_stdout = String::from_utf8_lossy(output.stdout.as_ref());
                    let re = Regex::new(needle_regex).unwrap();
                    re.captures_iter(cmd_stdout.as_ref())
                        .map(|capt| String::from(capt.get(1).expect("group 1 not found").as_str().trim()))
                        .collect()
                } else {
                    error!("Execution of ic-admin failed: {}", String::from_utf8_lossy(output.stderr.as_ref()));
                    vec![]
                }
            }
            Err(err) => {
                error!("Error starting ic-admin process: {}", err);
                vec![]
            }
        }
    }
}

#[derive(Display, Clone)]
#[strum(serialize_all = "kebab-case")]
pub enum ProposeCommand {
    ChangeSubnetMembership {
        subnet_id: PrincipalId,
        node_ids_add: Vec<PrincipalId>,
        node_ids_remove: Vec<PrincipalId>,
    },
    DeployGuestosToAllSubnetNodes {
        subnet: PrincipalId,
        version: String,
    },
    DeployGuestosToAllUnassignedNodes {
        replica_version: String,
    },
    DeployHostosToSomeNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    Raw {
        command: String,
        args: Vec<String>,
    },
    RemoveNodes {
        nodes: Vec<PrincipalId>,
    },
    ReviseElectedVersions {
        release_artifact: Artifact,
        args: Vec<String>,
    },
    CreateSubnet {
        node_ids: Vec<PrincipalId>,
        replica_version: String,
        other_args: Vec<String>,
    },
    AddApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    RemoveApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
    },
    DeployGuestosToSomeApiBoundaryNodes {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    SetAuthorizedSubnetworks {
        subnets: Vec<PrincipalId>,
    },
}

impl ProposeCommand {
    fn get_command_name(&self) -> String {
        const PROPOSE_CMD_PREFIX: &str = "propose-to-";
        format!(
            "{PROPOSE_CMD_PREFIX}{}",
            match self {
                Self::Raw { command, args: _ } => command.trim_start_matches(PROPOSE_CMD_PREFIX).to_string(),
                Self::ReviseElectedVersions { release_artifact, args: _ } => format!("revise-elected-{}-versions", release_artifact),
                Self::DeployGuestosToAllUnassignedNodes { replica_version: _ } => "deploy-guestos-to-all-unassigned-nodes".to_string(),
                _ => self.to_string(),
            }
        )
    }
}

impl ProposeCommand {
    fn args(&self) -> Vec<String> {
        match &self {
            Self::ChangeSubnetMembership {
                subnet_id,
                node_ids_add: nodes_ids_add,
                node_ids_remove: nodes_ids_remove,
            } => [
                vec!["--subnet-id".to_string(), subnet_id.to_string()],
                if !nodes_ids_add.is_empty() {
                    [
                        vec!["--node-ids-add".to_string()],
                        nodes_ids_add.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                    ]
                    .concat()
                } else {
                    vec![]
                },
                if !nodes_ids_remove.is_empty() {
                    [
                        vec!["--node-ids-remove".to_string()],
                        nodes_ids_remove.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                    ]
                    .concat()
                } else {
                    vec![]
                },
            ]
            .concat(),
            Self::DeployGuestosToAllSubnetNodes { subnet, version } => {
                vec![subnet.to_string(), version.clone()]
            }
            Self::Raw { command: _, args } => args.clone(),
            Self::DeployHostosToSomeNodes { nodes, version } => [
                nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec!["--hostos-version-id".to_string(), version.to_string()],
            ]
            .concat(),
            Self::RemoveNodes { nodes } => nodes.iter().map(|n| n.to_string()).collect(),
            Self::ReviseElectedVersions { release_artifact: _, args } => args.clone(),
            Self::CreateSubnet {
                node_ids,
                replica_version,
                other_args,
            } => {
                let mut args = vec!["--subnet-type".to_string(), "application".to_string()];

                args.push("--replica-version-id".to_string());
                args.push(replica_version.to_string());

                for id in node_ids {
                    args.push(id.to_string())
                }
                args.extend(other_args.to_vec());
                args
            }
            Self::DeployGuestosToAllUnassignedNodes { replica_version } => {
                vec!["--replica-version-id".to_string(), replica_version.clone()]
            }
            Self::AddApiBoundaryNodes { nodes, version } => [
                nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
                vec!["--version".to_string(), version.to_string()],
            ]
            .concat(),
            Self::RemoveApiBoundaryNodes { nodes } => nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
            Self::DeployGuestosToSomeApiBoundaryNodes { nodes, version } => [
                nodes.iter().flat_map(|n| ["--nodes".to_string(), n.to_string()]).collect::<Vec<_>>(),
                vec!["--version".to_string(), version.to_string()],
            ]
            .concat(),
            Self::SetAuthorizedSubnetworks { subnets } => subnets.iter().flat_map(|s| ["--subnets".to_string(), s.to_string()]).collect::<Vec<_>>(),
        }
    }
}

#[derive(Default, Clone)]
pub struct ProposeOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
    pub forum_post_link: Option<String>,
}
pub const FALLBACK_IC_ADMIN_VERSION: &str = "d4ee25b0865e89d3eaac13a60f0016d5e3296b31";

fn get_ic_admin_revisions_dir() -> anyhow::Result<PathBuf> {
    let dir = dirs::home_dir()
        .map(|d| d.join("bin").join("ic-admin.revisions"))
        .ok_or_else(|| anyhow::format_err!("Cannot find home directory"))?;

    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }

    Ok(dir)
}
const DURATION_BETWEEN_CHECKS: Duration = Duration::from_secs(60 * 60 * 24);
pub fn should_update_ic_admin() -> Result<(bool, String)> {
    let ic_admin_bin_dir = get_ic_admin_revisions_dir()?;
    let file = ic_admin_bin_dir.join("ic-admin.status");

    if !file.exists() {
        return Ok((true, "".to_string()));
    }

    let mut status_file = std::fs::File::open(&file)?;
    let elapsed = status_file.metadata()?.modified()?.elapsed().unwrap_or_default();
    if elapsed > DURATION_BETWEEN_CHECKS {
        info!("Checking if there is a new version of ic-admin");

        return Ok((true, "".to_string()));
    }
    let mut version = "".to_string();
    status_file.read_to_string(&mut version)?;
    info!("Using ic-admin {version} because lock file was created less than 24 hours ago");
    let path = ic_admin_bin_dir.join(&version).join("ic-admin");
    Ok((false, path.display().to_string()))
}

/// Returns a path to downloaded ic-admin binary
pub async fn download_ic_admin(version: Option<String>) -> Result<String> {
    let version = version.unwrap_or_else(|| FALLBACK_IC_ADMIN_VERSION.to_string()).trim().to_string();
    let ic_admin_bin_dir = get_ic_admin_revisions_dir()?;
    let path = ic_admin_bin_dir.join(&version).join("ic-admin");
    let path = Path::new(&path);

    if !path.exists() {
        let url = if std::env::consts::OS == "macos" {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-linux/ic-admin.gz")
        };
        info!("Downloading ic-admin version: {} from {}", version, url);
        let body = reqwest::get(url).await?.error_for_status()?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().expect("path parent unwrap failed!");
        std::fs::create_dir_all(path_parent).unwrap_or_else(|_| panic!("create_dir_all failed for {}", path_parent.display()));
        let mut out = std::fs::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::write(ic_admin_bin_dir.join("ic-admin.status"), version)?;
    info!("Using ic-admin: {}", path.display());

    Ok(path.to_string_lossy().to_string())
}
