use anyhow::{anyhow, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use futures::Future;
use ic_base_types::PrincipalId;
use log::{debug, error, info, warn};
use python_input::input;
use regex::Regex;
use std::os::unix::fs::PermissionsExt;
use std::{path::Path, process::Command};
use strum::Display;

use crate::cli::Opts;
use crate::defaults;

#[derive(Clone)]
pub struct CliDeprecated {
    ic_admin: Option<String>,
    nns_url: Option<String>,
    dry_run: bool,

    hsm_pin: Option<String>,
    hsm_slot: Option<String>,
    hsm_key_id: Option<String>,
    neuron_id: Option<u64>,
}

impl CliDeprecated {
    pub fn dry_run(&self) -> Self {
        Self {
            dry_run: true,
            ..self.clone()
        }
    }

    pub(crate) fn run(&self, command: &str, args: &[String]) -> Result<String> {
        let root_options = [
            self.hsm_pin
                .clone()
                .map(|p| vec!["--use-hsm".to_string(), "--pin".to_string(), p])
                .unwrap_or_default(),
            self.hsm_slot
                .clone()
                .map(|s| vec!["--slot".to_string(), s])
                .unwrap_or_default(),
            self.hsm_key_id
                .clone()
                .map(|k| vec!["--key-id".to_string(), k])
                .unwrap_or_default(),
            self.nns_url
                .clone()
                .map(|n| vec!["--nns-url".to_string(), n])
                .unwrap_or_default(),
        ]
        .concat();

        let ic_admin_args = [root_options.as_slice(), &[command.to_string()], args].concat();

        let ic_admin_path = self.ic_admin.as_ref().unwrap();
        fn print_ic_admin_command_line(cmd: &Command) {
            println!(
                "$ {} {}",
                cmd.get_program().to_str().unwrap().yellow(),
                shlex::join(cmd.get_args().map(|s| s.to_str().unwrap()).collect::<Vec<_>>()).yellow()
            );
        }

        let mut cmd = Command::new(ic_admin_path);
        let cmd = cmd.args(ic_admin_args);
        if !self.dry_run {
            info!("Running the ic-admin command");
            print_ic_admin_command_line(cmd);

            let output = cmd.output()?;
            let stdout = String::from_utf8_lossy(output.stdout.as_ref()).to_string();
            debug!("STDOUT:\n{}", stdout);
            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(output.stderr.as_ref()).to_string();
                warn!("STDERR:\n{}", stderr);
            }
            Ok(stdout)
        } else {
            println!("Please confirm enqueueing the following ic-admin command");
            // Show the user the line that would be executed and let them decide if they
            // want to proceed.
            print_ic_admin_command_line(cmd);

            let buffer = input("Would you like to proceed [y/N]? ");
            if let "Y" | "YES" = buffer.to_uppercase().as_str() {
                Ok("User confirmed".to_string())
            } else {
                Err(anyhow!("Cancelling operation, user entered '{}'", buffer.as_str(),))
            }
        }
    }

    pub(crate) fn propose_run(
        &self,
        cmd: ProposeCommand,
        ProposeOptions {
            title,
            summary,
            motivation: _,
        }: ProposeOptions,
    ) -> Result<String> {
        self.run(
            &format!("propose-to-{}", cmd),
            [
                title
                    .map(|t| vec!["--proposal-title".to_string(), t])
                    .unwrap_or_default(),
                summary.map(|s| vec!["--summary".to_string(), s]).unwrap_or_default(),
                self.neuron_id
                    .map(|n| vec!["--proposer".to_string(), n.to_string()])
                    .unwrap_or_default(),
                cmd.args(),
            ]
            .concat()
            .as_slice(),
        )
    }
}

#[derive(Clone, Default)]
pub struct Cli {
    ic_admin: Option<String>,
    nns_url: Option<String>,
    pub dry_run: bool,
    neuron: Option<Neuron>,
}

#[derive(Clone)]
pub struct Neuron {
    id: u64,
    auth: Auth,
}

impl Neuron {
    pub fn as_arg_vec(&self) -> Vec<String> {
        vec!["--proposer".to_string(), self.id.to_string()]
    }
}

#[derive(Clone)]
pub enum Auth {
    Hsm { pin: String, slot: String, key_id: String },
    Keyfile { path: String },
}

impl Auth {
    pub fn as_arg_vec(&self) -> Vec<String> {
        match self {
            Auth::Hsm { pin, slot, key_id } => vec![
                "--use-hsm".to_string(),
                "--pin".to_string(),
                pin.clone(),
                "--slot".to_string(),
                slot.clone(),
                "--key-id".to_string(),
                key_id.clone(),
            ],
            Auth::Keyfile { path } => vec!["--secret-key-pem".to_string(), path.clone()],
        }
    }
}

impl Cli {
    pub fn dry_run(&self) -> Self {
        Self {
            dry_run: true,
            ..self.clone()
        }
    }

    fn print_ic_admin_command_line(cmd: &Command) {
        info!(
            "running ic-admin: \n$ {}{}",
            cmd.get_program().to_str().unwrap().yellow(),
            cmd.get_args()
                .map(|s| s.to_str().unwrap().to_string())
                .fold("".to_string(), |acc, s| {
                    let s = if s.contains('\n') { format!(r#""{}""#, s) } else { s };
                    if s.starts_with("--") {
                        format!("{acc} \\\n    {s}")
                    } else if !acc.split(' ').last().unwrap_or_default().starts_with("--") {
                        format!("{acc} \\\n  {s}")
                    } else {
                        format!("{acc} {s}")
                    }
                })
                .yellow(),
        );
    }

    pub(crate) fn propose_run(
        &self,
        cmd: ProposeCommand,
        ProposeOptions {
            title,
            summary,
            motivation,
        }: ProposeOptions,
    ) -> anyhow::Result<()> {
        self.run(
            &format!("propose-to-{}", cmd),
            [
                if self.dry_run {
                    vec!["--dry-run".to_string()]
                } else {
                    Default::default()
                },
                title
                    .map(|t| vec!["--proposal-title".to_string(), t])
                    .unwrap_or_default(),
                summary
                    .map(|s| {
                        vec![
                            "--summary".to_string(),
                            format!(
                                "{}{}",
                                s,
                                motivation.map(|m| format!("\n\nMotivation: {m}")).unwrap_or_default(),
                            ),
                        ]
                    })
                    .unwrap_or_default(),
                self.neuron
                    .as_ref()
                    .map(|n| n.as_arg_vec())
                    .ok_or_else(|| anyhow::anyhow!("cannot submit a proposal without a neuron ID"))?,
                cmd.args(),
            ]
            .concat()
            .as_slice(),
        )
    }

    fn _run_ic_admin_with_args(&self, ic_admin_args: &[String]) -> anyhow::Result<()> {
        let ic_admin_path = self.ic_admin.clone().unwrap_or_else(|| "ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let cmd = cmd.args(ic_admin_args);

        Self::print_ic_admin_command_line(cmd);

        match cmd.spawn() {
            Ok(mut child) => match child.wait() {
                Ok(s) => {
                    if s.success() {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!(
                            "ic-admin failed with non-zero exit code {}",
                            s.code().map(|c| c.to_string()).unwrap_or_else(|| "<none>".to_string())
                        ))
                    }
                }
                Err(err) => Err(anyhow::format_err!("ic-admin wasn't running: {}", err.to_string())),
            },
            Err(e) => Err(anyhow::format_err!("failed to run ic-admin: {}", e.to_string())),
        }
    }

    pub(crate) fn run(&self, command: &str, args: &[String]) -> anyhow::Result<()> {
        let root_options = [
            self.neuron.as_ref().map(|n| n.auth.as_arg_vec()).unwrap_or_default(),
            self.nns_url
                .clone()
                .map(|n| vec!["--nns-url".to_string(), n])
                .unwrap_or_default(),
        ]
        .concat();

        let ic_admin_args = [root_options.as_slice(), &[command.to_string()], args].concat();

        self._run_ic_admin_with_args(&ic_admin_args)
    }

    /// Run ic-admin and parse sub-commands that it lists with "--help",
    /// extract the ones matching `needle_regex` and return them as a
    /// `Vec<String>`
    fn grep_subcommands(&self, needle_regex: &str) -> Vec<String> {
        let ic_admin_path = self.ic_admin.clone().unwrap_or_else(|| "ic-admin".to_string());
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
                    error!(
                        "Execution of ic-admin failed: {}",
                        String::from_utf8_lossy(output.stderr.as_ref())
                    );
                    vec![]
                }
            }
            Err(err) => {
                error!("Error starting ic-admin process: {}", err);
                vec![]
            }
        }
    }

    /// Run an `ic-admin get-*` command directly, and without an HSM
    pub(crate) fn run_passthrough_get(&self, args: &[String]) -> anyhow::Result<()> {
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
            // i.e., `release_cli get get-subnet 0` still works, although `release_cli get
            // subnet 0` is preferred
            args.to_vec()
        } else {
            // But since ic-admin expects these commands to include the "get-" prefix, we
            // need to add it back Example:
            // `release_cli get subnet 0` becomes
            // `ic-admin --nns-url "http://[2600:3004:1200:1200:5000:7dff:fe29:a2f5]:8080" get-subnet 0`
            let mut args_with_get_prefix = vec![String::from("get-") + args[0].as_str()];
            args_with_get_prefix.extend_from_slice(args.split_at(1).1);
            args_with_get_prefix
        };

        let root_options = match &self.nns_url {
            Some(nns_url) => {
                vec!["--nns-url".to_string(), nns_url.clone()]
            }
            None => {
                warn!("NNS URL is not specified. Please specify it with --nns-url or env variable or an entry in .env file. Falling back to the staging NNS.");
                vec!["--nns-url".to_string(), defaults::DEFAULT_NNS_URL.to_string()]
            }
        };

        let ic_admin_args = [root_options.as_slice(), args.as_slice()].concat();

        self._run_ic_admin_with_args(&ic_admin_args)
    }

    /// Run an `ic-admin propose-to-*` command directly
    pub(crate) fn run_passthrough_propose(&self, args: &[String]) -> anyhow::Result<()> {
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

        let nns_args = match &self.nns_url {
            Some(nns_url) => vec!["--nns-url".to_string(), nns_url.clone()],
            None => {
                warn!("NNS URL is not specified. Please specify it with --nns-url or env variable or an entry in .env file. Falling back to the staging NNS.");
                vec!["--nns-url".to_string(), defaults::DEFAULT_NNS_URL.to_string()]
            }
        };
        let root_options = [
            nns_args,
            self.neuron.as_ref().map(|n| n.auth.as_arg_vec()).unwrap_or_default(),
        ]
        .concat();

        let ic_admin_args = [root_options.as_slice(), args.as_slice()].concat();

        self._run_ic_admin_with_args(&ic_admin_args)
    }
}

#[derive(Display)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum ProposeCommand {
    AddNodesToSubnet {
        subnet_id: PrincipalId,
        nodes: Vec<PrincipalId>,
    },
    RemoveNodesFromSubnet {
        nodes: Vec<PrincipalId>,
    },
    UpdateSubnetReplicaVersion {
        subnet: PrincipalId,
        version: String,
    },
}

impl ProposeCommand {
    fn args(&self) -> Vec<String> {
        match &self {
            Self::AddNodesToSubnet { subnet_id, nodes } => vec![
                nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec!["--subnet-id".to_string(), subnet_id.to_string()],
            ]
            .concat(),
            Self::RemoveNodesFromSubnet { nodes } => nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
            Self::UpdateSubnetReplicaVersion { subnet, version } => {
                vec![subnet.to_string(), version.clone()]
            }
        }
    }
}

#[derive(Default)]
pub struct ProposeOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
}

impl From<&Opts> for CliDeprecated {
    fn from(opts: &Opts) -> Self {
        CliDeprecated {
            hsm_pin: opts.hsm_pin.clone(),
            hsm_slot: opts.hsm_slot.clone(),
            hsm_key_id: opts.hsm_key_id.clone(),
            neuron_id: opts.neuron_id,
            ic_admin: opts.ic_admin.clone(),
            nns_url: opts.nns_url.clone(),
            dry_run: opts.dry_run,
        }
    }
}

impl From<&Opts> for Cli {
    fn from(opts: &Opts) -> Self {
        Cli {
            neuron: Neuron {
                id: opts.neuron_id.unwrap_or_default(),
                auth: match &opts.private_key_pem {
                    Some(private_key_pem) => Auth::Keyfile {
                        path: private_key_pem.clone(),
                    },
                    None => Auth::Hsm {
                        pin: opts.hsm_pin.clone().unwrap_or_default(),
                        slot: opts.hsm_slot.clone().unwrap_or_default(),
                        key_id: opts.hsm_key_id.clone().unwrap_or_default(),
                    },
                },
            }
            .into(),
            ic_admin: opts.ic_admin.clone(),
            nns_url: opts.nns_url.clone(),
            dry_run: opts.dry_run,
        }
    }
}

/// Returns a path to downloaded ic-admin binary
async fn download_ic_admin(version: Option<String>) -> Result<String> {
    let version = version.unwrap_or_else(|| defaults::DEFAULT_IC_ADMIN_VERSION.to_string());

    let home_dir = dirs::home_dir()
        .and_then(|d| d.to_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow::format_err!("Cannot find home directory"))?;
    let path = format!("{home_dir}/bin/ic-admin.revisions/{version}/ic-admin");
    let path = Path::new(&path);

    if !path.exists() {
        let url = if std::env::consts::OS == "macos" {
            format!("https://download.dfinity.systems/blessed/ic/{version}/nix-release/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/blessed/ic/{version}/release/ic-admin.gz")
        };
        let body = reqwest::get(url).await?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().expect("path parent unwrap failed!");
        std::fs::create_dir_all(path_parent)
            .unwrap_or_else(|_| panic!("create_dir_all failed for {}", path_parent.display()));
        let mut out = std::fs::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(path.to_string_lossy().to_string())
}

pub async fn with_ic_admin<F, U>(version: Option<String>, closure: F) -> Result<U>
where
    F: Future<Output = Result<U>>,
{
    let ic_admin_path = download_ic_admin(version).await?;
    let bin_dir = Path::new(&ic_admin_path).parent().unwrap();
    std::env::set_var(
        "PATH",
        format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
    );

    closure.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_propose_dry_run() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"-----BEGIN PRIVATE KEY-----
MFMCAQEwBQYDK2VwBCIEIB/tIlNK+7Knr2GuhIyzu1Z0bOcDwJqtSzvKDAXxFfac
oSMDIQBa2NLmSmaqjDXej4rrJEuEhKIz7/pGXpxztViWhB+X9Q==
-----END PRIVATE KEY-----"#
        )?;

        let test_cases = vec![
            ProposeCommand::AddNodesToSubnet {
                subnet_id: Default::default(),
                nodes: vec![Default::default()],
            },
            ProposeCommand::RemoveNodesFromSubnet {
                nodes: vec![Default::default()],
            },
            ProposeCommand::UpdateSubnetReplicaVersion {
                subnet: Default::default(),
                version: "0000000000000000000000000000000000000000".to_string(),
            },
        ];

        for cmd in test_cases {
            let cli = Cli {
                nns_url: "http://localhost:8080".to_string().into(),
                dry_run: true,
                neuron: Neuron {
                    id: 3,
                    auth: Auth::Keyfile {
                        path: file
                            .path()
                            .to_str()
                            .ok_or_else(|| anyhow::format_err!("Could not convert temp file path to string"))?
                            .to_string(),
                    },
                }
                .into(),
                ..Default::default()
            };

            let cmd_name = cmd.to_string();
            let out = with_ic_admin(Default::default(), async {
                cli.propose_run(cmd, Default::default()).map_err(|e| anyhow::anyhow!(e))
            })
            .await;
            assert!(
                out.is_ok(),
                r#"failed running the ic-admin command for {cmd_name} subcommand: {}"#,
                out.err().map(|e| e.to_string()).unwrap_or_default()
            );
        }

        Ok(())
    }
}
