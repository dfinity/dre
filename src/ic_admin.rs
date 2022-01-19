use anyhow::{anyhow, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use ic_base_types::PrincipalId;
use log::{debug, info, warn};
use python_input::input;
use std::os::unix::fs::PermissionsExt;
use std::{path::Path, process::Command};
use strum::Display;

use crate::cli::Opts;

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
            print_ic_admin_command_line(&cmd);

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
            print_ic_admin_command_line(&cmd);

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
        ProposeOptions { title, summary }: ProposeOptions,
    ) -> Result<String> {
        self.run(
            &format!("propose-to-{}", cmd.to_string()),
            [
                title
                    .map(|t| vec!["--proposal-title".to_string(), t])
                    .unwrap_or_default(),
                summary.map(|s| vec!["--summary".to_string(), s]).unwrap_or_default(),
                self.neuron_id
                    .clone()
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
    dry_run: bool,
    neuron: Option<Neuron>,
}

#[derive(Clone)]
pub struct Neuron {
    id: u64,
    auth: Auth,
}

#[derive(Clone)]
pub enum Auth {
    HSM { pin: String, slot: String, key_id: String },
    Keyfile { path: String },
}

impl Cli {
    fn print_ic_admin_command_line(cmd: &Command) {
        info!(
            "running ic-admin: \n$ {} {}",
            cmd.get_program().to_str().unwrap().yellow(),
            shlex::join(cmd.get_args().map(|s| s.to_str().unwrap()).collect::<Vec<_>>()).yellow()
        );
    }

    pub(crate) fn propose_run(
        &self,
        cmd: ProposeCommand,
        ProposeOptions { title, summary }: ProposeOptions,
    ) -> Result<String, String> {
        self.run(
            &format!("propose-to-{}", cmd.to_string()),
            [
                if self.dry_run {
                    vec!["--dry-run".to_string()]
                } else {
                    Default::default()
                },
                title
                    .map(|t| vec!["--proposal-title".to_string(), t])
                    .unwrap_or_default(),
                summary.map(|s| vec!["--summary".to_string(), s]).unwrap_or_default(),
                self.neuron
                    .as_ref()
                    .map(|n| vec!["--proposer".to_string(), n.id.to_string()])
                    .ok_or("cannot submit a proposal without a neuron ID".to_string())?,
                cmd.args(),
            ]
            .concat()
            .as_slice(),
        )
    }

    pub(crate) fn run(&self, command: &str, args: &[String]) -> Result<String, String> {
        let root_options = [
            self.neuron
                .as_ref()
                .map(|n| match n.auth.clone() {
                    Auth::HSM { pin, slot, key_id } => vec![
                        "--use-hsm".to_string(),
                        "--pin".to_string(),
                        pin,
                        "--slot".to_string(),
                        slot,
                        "--key-id".to_string(),
                        key_id,
                    ],
                    Auth::Keyfile { path } => vec!["-s".to_string(), path],
                })
                .unwrap_or_default(),
            self.nns_url
                .clone()
                .map(|n| vec!["--nns-url".to_string(), n])
                .unwrap_or_default(),
        ]
        .concat();

        let ic_admin_args = [root_options.as_slice(), &[command.to_string()], args].concat();
        let ic_admin_path = self.ic_admin.clone().unwrap_or("ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let cmd = cmd.args(ic_admin_args);

        Self::print_ic_admin_command_line(&cmd);

        let output = cmd.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            Err(String::from_utf8_lossy(output.stderr.as_ref()).to_string())
        } else {
            Ok(String::from_utf8_lossy(output.stdout.as_ref()).to_string())
        }
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
}

impl From<&Opts> for CliDeprecated {
    fn from(opts: &Opts) -> Self {
        CliDeprecated {
            hsm_pin: opts.hsm_pin.clone(),
            hsm_slot: opts.hsm_slot.clone(),
            hsm_key_id: opts.hsm_key_id.clone(),
            neuron_id: opts.neuron_id.clone(),
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
                id: opts.neuron_id.clone().unwrap(),
                auth: Auth::HSM {
                    pin: opts.hsm_pin.clone().unwrap(),
                    slot: opts.hsm_slot.clone().unwrap(),
                    key_id: opts.hsm_key_id.clone().unwrap(),
                },
            }
            .into(),
            ic_admin: opts.ic_admin.clone(),
            nns_url: opts.nns_url.clone(),
            dry_run: opts.dry_run,
        }
    }
}

const DEFAULT_IC_ADMIN_VERSION: &str = "936bf9ccaabd566c68232e5cb3f3ce7d5ae89328";

/// Returns a path to downloaded ic-admin binary
fn download_ic_admin(version: Option<String>) -> Result<String> {
    let version = version.unwrap_or_else(|| DEFAULT_IC_ADMIN_VERSION.to_string());

    let home_dir = dirs::home_dir()
        .and_then(|d| d.to_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow::format_err!("Cannot find home directory"))?;
    let path = format!("{home_dir}/bin/ic-admin/{version}/ic-admin");
    let path = Path::new(&path);

    if !path.exists() {
        let url = if std::env::consts::OS == "macos" {
            format!("https://download.dfinity.systems/blessed/ic/{version}/nix-release/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/blessed/ic/{version}/release/ic-admin.gz")
        };
        let resp = reqwest::blocking::get(url)?;
        let mut decoded = GzDecoder::new(resp);

        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut out = std::fs::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(path.to_string_lossy().to_string())
}

pub fn with_ic_admin<T, U>(version: Option<String>, closure: T) -> Result<U>
where
    T: Fn() -> Result<U>,
{
    let ic_admin_path = download_ic_admin(version)?;
    let bin_dir = Path::new(&ic_admin_path).parent().unwrap();
    std::env::set_var(
        "PATH",
        format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
    );

    closure()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_propose_dry_run() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"-----BEGIN PRIVATE KEY-----
MFMCAQEwBQYDK2VwBCIEIB/tIlNK+7Knr2GuhIyzu1Z0bOcDwJqtSzvKDAXxFfac
oSMDIQBa2NLmSmaqjDXej4rrJEuEhKIz7/pGXpxztViWhB+X9Q==
-----END PRIVATE KEY-----"#
        )?;

        let cli = Cli {
            nns_url: "http://localhost:8080".to_string().into(),
            dry_run: true,
            neuron: Neuron {
                id: 3,
                auth: Auth::Keyfile {
                    path: file
                        .path()
                        .to_str()
                        .ok_or(anyhow::format_err!("Could not convert temp file path to string"))?
                        .to_string(),
                },
            }
            .into(),
            ..Default::default()
        };

        let out = with_ic_admin(Default::default(), || {
            cli.propose_run(
                ProposeCommand::RemoveNodesFromSubnet {
                    nodes: vec![Default::default()],
                },
                Default::default(),
            )
            .map_err(|e| anyhow::anyhow!(e))
        });
        assert!(
            out.is_ok(),
            r#"failed running the ic-admin command: {}"#,
            out.err().map(|e| e.to_string()).unwrap_or_default()
        );

        let out = out.unwrap();
        assert_eq!(
            out,
            r#"submit_proposal payload: 
{"node_ids":["aaaaa-aa"]}
"#
        );
        Ok(())
    }
}
