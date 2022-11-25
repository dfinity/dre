use anyhow::Result;
use candid::{Decode, Encode};
use colored::Colorize;
use cryptoki::context::{CInitializeArgs, Pkcs11};
use cryptoki::session::{SessionFlags, UserType};
use dialoguer::console::Term;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Password, Select};
use flate2::read::GzDecoder;
use futures::Future;
use ic_base_types::PrincipalId;
use ic_canister_client::{Agent, Sender};
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance::pb::v1::{ListNeurons, ListNeuronsResponse};
use ic_sys::utility_command::UtilityCommand;
use keyring::{Entry, Error};
use log::{error, info, warn};
use regex::Regex;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::str::FromStr;
use std::{path::Path, process::Command};
use strum::Display;

use crate::cli::Opts;
use crate::defaults;

#[derive(Clone)]
pub struct Cli {
    ic_admin: Option<String>,
    nns_url: url::Url,
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
    Hsm { pin: String, slot: u64, key_id: String },
    Keyfile { path: String },
}

fn pkcs11_lib_path() -> anyhow::Result<PathBuf> {
    let lib_macos_path = PathBuf::from_str("/Library/OpenSC/lib/opensc-pkcs11.so")?;
    let lib_linux_path = PathBuf::from_str("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so")?;
    if lib_macos_path.exists() {
        Ok(lib_macos_path)
    } else if lib_linux_path.exists() {
        Ok(lib_linux_path)
    } else {
        Err(anyhow::anyhow!("no pkcs11 library found"))
    }
}

fn get_pkcs11_ctx() -> anyhow::Result<Pkcs11> {
    let pkcs11 = Pkcs11::new(pkcs11_lib_path()?)?;
    pkcs11.initialize(CInitializeArgs::OsThreads)?;
    Ok(pkcs11)
}

impl Auth {
    pub fn as_arg_vec(&self) -> Vec<String> {
        match self {
            Auth::Hsm { pin, slot, key_id } => vec![
                "--use-hsm".to_string(),
                "--pin".to_string(),
                pin.clone(),
                "--slot".to_string(),
                slot.to_string(),
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

    fn print_ic_admin_command_line(&self, cmd: &Command) {
        info!(
            "running ic-admin: \n$ {}{}",
            cmd.get_program().to_str().unwrap().yellow(),
            cmd.get_args()
                .map(|s| s.to_str().unwrap().to_string())
                .fold("".to_string(), |acc, s| {
                    let s = if s.contains('\n') { format!(r#""{}""#, s) } else { s };
                    if self
                        .neuron
                        .as_ref()
                        .and_then(|n| {
                            if let Auth::Hsm { pin, .. } = &n.auth {
                                Some(pin.clone())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                        == s
                    {
                        format!("{acc} <redacted>")
                    } else if s.starts_with("--") {
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
            &cmd.get_command_name(),
            [
                if self.dry_run || self.neuron.is_none() {
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
                self.neuron.as_ref().map(|n| n.as_arg_vec()).unwrap_or_default(),
                cmd.args(),
            ]
            .concat()
            .as_slice(),
        )
    }

    fn _run_ic_admin_with_args(&self, ic_admin_args: &[String]) -> anyhow::Result<()> {
        let ic_admin_path = self.ic_admin.clone().unwrap_or_else(|| "ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let root_options = [
            self.neuron.as_ref().map(|n| n.auth.as_arg_vec()).unwrap_or_default(),
            vec!["--nns-url".to_string(), self.nns_url.to_string()],
        ]
        .concat();
        let cmd = cmd.args([&root_options, ic_admin_args].concat());

        self.print_ic_admin_command_line(cmd);

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
        let ic_admin_args = [&[command.to_string()], args].concat();
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

        self.run(&args[0], &args.iter().skip(1).cloned().collect::<Vec<_>>())
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

        let exec = |cli: &Cli| {
            cli.propose_run(
                ProposeCommand::Raw {
                    command: args[0].clone(),
                    args: args.iter().skip(1).cloned().collect::<Vec<_>>(),
                },
                ProposeOptions::default(),
            )
        };
        if !self.dry_run {
            exec(&self.dry_run())?;
            if !Confirm::new()
                .with_prompt("Do you want to continue?")
                .default(false)
                .interact()?
            {
                return Err(anyhow::anyhow!("Action aborted"));
            }
        }
        exec(self)
    }
}

#[derive(Display)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum ProposeCommand {
    ChangeSubnetMembership {
        subnet_id: PrincipalId,
        node_ids_add: Vec<PrincipalId>,
        node_ids_remove: Vec<PrincipalId>,
    },
    UpdateSubnetReplicaVersion {
        subnet: PrincipalId,
        version: String,
    },
    Raw {
        command: String,
        args: Vec<String>,
    },
}

impl ProposeCommand {
    fn get_command_name(&self) -> String {
        const PROPOSE_CMD_PREFIX: &str = "propose-to-";
        format!(
            "{PROPOSE_CMD_PREFIX}{}",
            match self {
                Self::Raw { command, args: _ } => command.trim_start_matches(PROPOSE_CMD_PREFIX).to_string(),
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
            } => vec![
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
            Self::UpdateSubnetReplicaVersion { subnet, version } => {
                vec![subnet.to_string(), version.clone()]
            }
            Self::Raw { command: _, args } => args.clone(),
        }
    }
}

#[derive(Default)]
pub struct ProposeOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
}

fn detect_hsm_auth() -> Result<Option<Auth>> {
    info!("Detecting HSM devices");
    let ctx = get_pkcs11_ctx()?;
    for slot in ctx.get_slots_with_token()? {
        let info = ctx.get_slot_info(slot)?;
        if info.slot_description().starts_with("Nitrokey Nitrokey HSM") {
            let key_id = format!("hsm-{}-{}", info.slot_description(), info.manufacturer_id());
            let pin_entry = Entry::new("release-cli", &key_id);
            let pin = match pin_entry.get_password() {
                Err(Error::NoEntry) => Password::new().with_prompt("Please enter the HSM PIN: ").interact()?,
                Ok(pin) => pin,
                Err(e) => return Err(anyhow::anyhow!("Failed to get pin from keyring: {}", e)),
            };

            let mut flags = SessionFlags::new();
            flags.set_serial_session(true);
            let session = ctx.open_session_no_callback(slot, flags).unwrap();
            session.login(UserType::User, Some(&pin))?;
            info!("HSM login successful!");
            pin_entry.set_password(&pin)?;
            return Ok(Some(Auth::Hsm {
                pin,
                slot: slot.id(),
                key_id: "01".to_string(),
            }));
        }
    }
    Ok(None)
}

async fn detect_neuron(url: url::Url) -> anyhow::Result<Option<Neuron>> {
    if let Some(Auth::Hsm { pin, slot, key_id }) = detect_hsm_auth()? {
        let auth = Auth::Hsm {
            pin: pin.clone(),
            slot,
            key_id: key_id.clone(),
        };
        let sender = Sender::from_external_hsm(
            UtilityCommand::read_public_key(Some(&slot.to_string()), Some(&key_id)).execute()?,
            std::sync::Arc::new(move |input| {
                Ok(
                    UtilityCommand::sign_message(input.to_vec(), Some(&slot.to_string()), Some(&pin), Some(&key_id))
                        .execute()?,
                )
            }),
        );
        let agent = Agent::new(url, sender);
        let neuron_id = if let Some(response) = agent
            .execute_query(
                &GOVERNANCE_CANISTER_ID,
                "list_neurons",
                Encode!(&ListNeurons {
                    include_neurons_readable_by_caller: true,
                    neuron_ids: vec![],
                })?,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?
        {
            let response = Decode!(&response, ListNeuronsResponse)?;
            let neuron_ids = response.neuron_infos.keys().copied().collect::<Vec<_>>();
            match neuron_ids.len() {
                0 => return Err(anyhow::anyhow!("HSM doesn't control any neurons")),
                1 => neuron_ids[0],
                _ => Select::with_theme(&ColorfulTheme::default())
                    .items(&neuron_ids)
                    .default(0)
                    .interact_on_opt(&Term::stderr())?
                    .map(|i| neuron_ids[i])
                    .ok_or_else(|| anyhow::anyhow!("No neuron selected"))?,
            }
        } else {
            return Err(anyhow::anyhow!("Empty response when listing controlled neurons"));
        };

        Ok(Some(Neuron { id: neuron_id, auth }))
    } else {
        Ok(None)
    }
}

impl Cli {
    pub async fn from_opts(opts: &Opts, require_neuron: bool) -> anyhow::Result<Self> {
        let nns_url = opts.network.get_url();
        let neuron = if let Some(id) = opts.neuron_id {
            Some(Neuron {
                id,
                auth: if let Some(path) = opts.private_key_pem.clone() {
                    Auth::Keyfile { path }
                } else if let (Some(slot), Some(pin), Some(key_id)) =
                    (opts.hsm_slot, opts.hsm_pin.clone(), opts.hsm_key_id.clone())
                {
                    Auth::Hsm { pin, slot, key_id }
                } else {
                    detect_hsm_auth()?
                        .ok_or_else(|| anyhow::anyhow!("No valid authentication method found for neuron: {id}"))?
                },
            })
        } else if require_neuron {
            detect_neuron(nns_url.clone()).await.unwrap_or_else(|e| {
                warn!("Failed to detect neuron: {}", e);
                warn!("No authentication set");
                None
            })
        } else {
            None
        };
        Ok(Cli {
            dry_run: opts.dry_run || neuron.is_none(),
            neuron,
            ic_admin: opts.ic_admin.clone(),
            nns_url,
        })
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
            format!("https://download.dfinity.systems/ic/{version}/nix-release/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/ic/{version}/release/ic-admin.gz")
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
    use std::{io::Write, str::FromStr};
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
            ProposeCommand::ChangeSubnetMembership {
                subnet_id: Default::default(),
                node_ids_add: vec![Default::default()],
                node_ids_remove: vec![Default::default()],
            },
            ProposeCommand::UpdateSubnetReplicaVersion {
                subnet: Default::default(),
                version: "0000000000000000000000000000000000000000".to_string(),
            },
        ];

        for cmd in test_cases {
            let cli = Cli {
                nns_url: url::Url::from_str("http://localhost:8080").unwrap(),
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
                ic_admin: None,
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
