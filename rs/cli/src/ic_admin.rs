use anyhow::Result;
use cli::UpdateVersion;
use colored::Colorize;
use dialoguer::Confirm;
use flate2::read::GzDecoder;
use futures::stream::{self, StreamExt};
use futures::Future;
use ic_base_types::PrincipalId;
use ic_management_types::Artifact;
use itertools::Itertools;
use log::{error, info, warn};
use regex::Regex;
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::{path::Path, process::Command};
use strum::Display;

use crate::cli::Cli;
use crate::detect_neuron::{Auth, Neuron};
use crate::{cli, defaults};

#[derive(Clone)]
pub struct IcAdminWrapper {
    ic_admin: Option<String>,
    nns_url: url::Url,
    yes: bool,
    neuron: Option<Neuron>,
}

impl From<Cli> for IcAdminWrapper {
    fn from(cli: Cli) -> Self {
        Self {
            ic_admin: cli.ic_admin,
            nns_url: cli.nns_url,
            yes: cli.yes,
            neuron: cli.neuron,
        }
    }
}

impl IcAdminWrapper {
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

    pub(crate) fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions, simulate: bool) -> anyhow::Result<()> {
        let exec = |cli: &IcAdminWrapper, cmd: ProposeCommand, opts: ProposeOptions, add_dryrun_arg: bool| {
            cli.run(
                &cmd.get_command_name(),
                [
                    // Make sure there is no more than one `--dry-run` argument, or else ic-admin will complain.
                    if add_dryrun_arg && !cmd.args().contains(&String::from("--dry-run")) {
                        vec!["--dry-run".to_string()]
                    } else {
                        Default::default()
                    },
                    opts.title
                        .map(|t| vec!["--proposal-title".to_string(), t])
                        .unwrap_or_default(),
                    opts.summary
                        .map(|s| {
                            vec![
                                "--summary".to_string(),
                                format!(
                                    "{}{}",
                                    s,
                                    opts.motivation
                                        .map(|m| format!("\n\nMotivation: {m}"))
                                        .unwrap_or_default(),
                                ),
                            ]
                        })
                        .unwrap_or_default(),
                    cli.neuron.as_ref().map(|n| n.as_arg_vec()).unwrap_or_default(),
                    cmd.args(),
                ]
                .concat()
                .as_slice(),
                true,
            )
        };

        // Simulated, or --help executions run immediately and do not proceed.
        if simulate || cmd.args().contains(&String::from("--help")) || cmd.args().contains(&String::from("--dry-run")) {
            return exec(self, cmd, opts, simulate);
        }

        // If --yes was not specified, ask the user if they want to proceed
        if !self.yes {
            exec(self, cmd.clone(), opts.clone(), true)?;
        }

        // User wants to proceed but does not have neuron configuration. Bail out.
        if self.neuron.is_none() {
            return Err(anyhow::anyhow!("Submitting this proposal requires a neuron, which was not detected -- and would cause ic-admin to fail during submition. Please look through your scroll buffer for specific error messages about your HSM and address the issue that prevents your neuron from being detected."));
        }

        if Confirm::new()
            .with_prompt("Do you want to continue?")
            .default(false)
            .interact()?
        {
            // User confirmed the desire to submit the proposal and no obvious problems were
            // found. Proceeding!
            exec(self, cmd, opts, false)
        } else {
            Err(anyhow::anyhow!("Action aborted"))
        }
    }

    fn _run_ic_admin_with_args(&self, ic_admin_args: &[String], with_auth: bool) -> anyhow::Result<()> {
        let ic_admin_path = self.ic_admin.clone().unwrap_or_else(|| "ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let auth_options = if with_auth {
            self.neuron.as_ref().map(|n| n.auth.as_arg_vec()).unwrap_or_default()
        } else {
            vec![]
        };
        let root_options = [auth_options, vec!["--nns-url".to_string(), self.nns_url.to_string()]].concat();
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

    pub(crate) fn run(&self, command: &str, args: &[String], with_auth: bool) -> anyhow::Result<()> {
        let ic_admin_args = [&[command.to_string()], args].concat();
        self._run_ic_admin_with_args(&ic_admin_args, with_auth)
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
            // `ic-admin --nns-url "http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080" get-subnet 0`
            let mut args_with_get_prefix = vec![String::from("get-") + args[0].as_str()];
            args_with_get_prefix.extend_from_slice(args.split_at(1).1);
            args_with_get_prefix
        };

        self.run(&args[0], &args.iter().skip(1).cloned().collect::<Vec<_>>(), false)
    }

    /// Run an `ic-admin propose-to-*` command directly
    pub(crate) fn run_passthrough_propose(&self, args: &[String], simulate: bool) -> anyhow::Result<()> {
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
                .map(|arg| {
                    if arg == "--motivation" {
                        "--summary".to_string()
                    } else {
                        arg.clone()
                    }
                })
                .collect::<Vec<_>>()
        } else {
            args.to_vec()
        };

        let cmd = ProposeCommand::Raw {
            command: args[0].clone(),
            args: args.iter().skip(1).cloned().collect::<Vec<_>>(),
        };
        let simulate = simulate || cmd.args().contains(&String::from("--dry-run"));
        self.propose_run(cmd, Default::default(), simulate)
    }

    fn get_s3_cdn_image_url(version: &String, s3_subdir: &String) -> String {
        format!(
            "https://download.dfinity.systems/ic/{}/{}/update-img/update-img.tar.gz",
            version, s3_subdir
        )
    }

    fn get_r2_cdn_image_url(version: &String, s3_subdir: &String) -> String {
        format!(
            "https://download.dfinity.network/ic/{}/{}/update-img/update-img.tar.gz",
            version, s3_subdir
        )
    }

    async fn download_file_and_get_sha256(download_url: &String) -> anyhow::Result<String> {
        let url = url::Url::parse(download_url)?;
        let subdir = format!(
            "{}{}",
            url.domain().expect("url.domain() is None"),
            url.path().to_owned()
        );
        // replace special characters in subdir with _
        let subdir = subdir.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        let download_dir = format!(
            "{}/tmp/ic/{}",
            dirs::home_dir().expect("home_dir is not set").as_path().display(),
            subdir
        );
        let download_dir = Path::new(&download_dir);

        std::fs::create_dir_all(download_dir)
            .unwrap_or_else(|_| panic!("create_dir_all failed for {}", download_dir.display()));

        let download_image = format!("{}/update-img.tar.gz", download_dir.to_str().unwrap());
        let download_image = Path::new(&download_image);

        let response = reqwest::get(download_url.clone()).await?;

        if response.status() != StatusCode::RANGE_NOT_SATISFIABLE && !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed with http_code {} for {}",
                response.status(),
                download_url
            ));
        }
        info!("Download {} succeeded {}", download_url, response.status());

        let mut file = match File::create(download_image) {
            Ok(file) => file,
            Err(err) => return Err(anyhow::anyhow!("Couldn't create a file: {}", err)),
        };

        let content = response.bytes().await?;
        file.write_all(&content)?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        let stringified_hash = hash[..]
            .iter()
            .map(|byte| format!("{:01$x?}", byte, 2))
            .collect::<Vec<String>>()
            .join("");
        info!(
            "File saved at {} has sha256 {}",
            download_image.display(),
            stringified_hash
        );
        Ok(stringified_hash)
    }

    async fn download_images_and_validate_sha256(
        image: &Artifact,
        version: &String,
    ) -> anyhow::Result<(Vec<String>, String)> {
        let update_urls = vec![
            Self::get_s3_cdn_image_url(version, &image.s3_folder()),
            Self::get_r2_cdn_image_url(version, &image.s3_folder()),
        ];

        // Download images, verify them and compare the SHA256
        let hash_and_valid_urls: Vec<(String, &String)> = stream::iter(&update_urls)
            .filter_map(|update_url| async move {
                match Self::download_file_and_get_sha256(update_url).await {
                    Ok(hash) => {
                        info!("SHA256 of {}: {}", update_url, hash);
                        Some((hash, update_url))
                    }
                    Err(err) => {
                        warn!("Error downloading {}: {}", update_url, err);
                        None
                    }
                }
            })
            .collect()
            .await;
        let hashes_unique = hash_and_valid_urls
            .iter()
            .map(|(h, _)| h.clone())
            .unique()
            .collect::<Vec<String>>();
        let expected_hash: String = match hashes_unique.len() {
            0 => {
                return Err(anyhow::anyhow!(
                    "Unable to download the update image from none of the following URLs: {}",
                    update_urls.join(", ")
                ))
            }
            1 => {
                let hash = hashes_unique.into_iter().next().unwrap();
                info!("SHA256 of all download images is: {}", hash);
                hash
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Update images do not have the same hash: {:?}",
                    hash_and_valid_urls
                        .iter()
                        .map(|(h, u)| format!("{}  {}", h, u))
                        .join("\n")
                ))
            }
        };
        let update_urls = hash_and_valid_urls
            .into_iter()
            .map(|(_, u)| u.clone())
            .collect::<Vec<String>>();

        if update_urls.is_empty() {
            return Err(anyhow::anyhow!(
                "Unable to download the update image from none of the following URLs: {}",
                update_urls.join(", ")
            ));
        } else if update_urls.len() == 1 {
            warn!("Only 1 update image is available. At least 2 should be present in the proposal");
        }
        Ok((update_urls, expected_hash))
    }

    pub(crate) async fn prepare_to_propose_to_update_elected_versions(
        release_artifact: &Artifact,
        version: &String,
        release_tag: &String,
        retire_versions: Option<Vec<String>>,
    ) -> anyhow::Result<UpdateVersion> {
        let (update_urls, expected_hash) = Self::download_images_and_validate_sha256(release_artifact, version).await?;

        let template = format!(
            r#"Elect new {release_artifact} binary revision [{version}](https://github.com/dfinity/ic/tree/{release_tag})

# Release Notes:

[comment]: <> Remove this block of text from the proposal.
[comment]: <> Then, add the {release_artifact} binary release notes as bullet points here.
[comment]: <> Any [commit ID] within square brackets will auto-link to the specific changeset.

# IC-OS Verification

To build and verify the IC-OS disk image, run:

```
# From https://github.com/dfinity/ic#verifying-releases
sudo apt-get install -y curl && curl --proto '=https' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/{version}/gitlab-ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c {version}
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image,
must be identical, and must match the SHA256 from the payload of the NNS proposal.
"#
        );
        let edited = edit::edit(template)?
            .trim()
            .replace("\r(\n)?", "\n")
            .split('\n')
            .map(|f| {
                if !f.starts_with('*') {
                    return f.to_string();
                }
                match f.split_once(']') {
                    Some((left, message)) => {
                        let commit_hash = left.split_once('[').unwrap().1.to_string();

                        format!(
                            "* [[{}](https://github.com/dfinity/ic/commit/{})] {}",
                            commit_hash, commit_hash, message
                        )
                    }
                    None => f.to_string(),
                }
            })
            .join("\n");
        if edited.contains(&String::from("Remove this block of text from the proposal.")) {
            Err(anyhow::anyhow!(
                "The edited proposal text has not been edited to add release notes."
            ))
        } else {
            let proposal_title = match &retire_versions {
                Some(v) => {
                    let pluralize = if v.len() == 1 { "version" } else { "versions" };
                    format!(
                        "Elect new IC/{} revision (commit {}), and retire old replica {} {}",
                        release_artifact.capitalized(),
                        &version[..8],
                        pluralize,
                        v.iter().map(|v| &v[..8]).join(",")
                    )
                }
                None => format!(
                    "Elect new IC/{} revision (commit {})",
                    release_artifact.capitalized(),
                    &version[..8]
                ),
            };

            Ok(UpdateVersion {
                release_artifact: release_artifact.clone(),
                version: version.clone(),
                title: proposal_title.clone(),
                stringified_hash: expected_hash,
                summary: edited,
                update_urls,
                versions_to_retire: retire_versions.clone(),
            })
        }
    }
}

#[derive(Display, Clone)]
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
    UpdateNodesHostosVersion {
        nodes: Vec<PrincipalId>,
        version: String,
    },
    RemoveNodes {
        nodes: Vec<PrincipalId>,
    },
    UpdateElectedVersions {
        release_artifact: Artifact,
        args: Vec<String>,
    },
    CreateSubnet {
        node_ids: Vec<PrincipalId>,
        replica_version: String,
    },
}

impl ProposeCommand {
    fn get_command_name(&self) -> String {
        const PROPOSE_CMD_PREFIX: &str = "propose-to-";
        format!(
            "{PROPOSE_CMD_PREFIX}{}",
            match self {
                Self::Raw { command, args: _ } => command.trim_start_matches(PROPOSE_CMD_PREFIX).to_string(),
                Self::UpdateElectedVersions {
                    release_artifact,
                    args: _,
                } => format!("update-elected-{}-versions", release_artifact),
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
            Self::UpdateNodesHostosVersion { nodes, version } => vec![
                nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec!["--hostos-version-id".to_string(), version.to_string()],
            ]
            .concat(),
            Self::RemoveNodes { nodes } => nodes.iter().map(|n| n.to_string()).collect(),
            Self::UpdateElectedVersions {
                release_artifact: _,
                args,
            } => args.clone(),
            Self::CreateSubnet {
                node_ids,
                replica_version,
            } => {
                let mut args = vec!["--subnet-type".to_string(), "application".to_string()];

                args.push("--replica-version-id".to_string());
                args.push(replica_version.to_string());

                for id in node_ids {
                    args.push(id.to_string())
                }
                args
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct ProposeOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub motivation: Option<String>,
}

/// Returns a path to downloaded ic-admin binary
async fn download_ic_admin(version: Option<String>) -> Result<String> {
    let version = version
        .unwrap_or_else(|| defaults::DEFAULT_IC_ADMIN_VERSION.to_string())
        .trim()
        .to_string();
    let home_dir = dirs::home_dir()
        .and_then(|d| d.to_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow::format_err!("Cannot find home directory"))?;
    let path = format!("{home_dir}/bin/ic-admin.revisions/{version}/ic-admin");
    let path = Path::new(&path);

    if !path.exists() {
        let url = if std::env::consts::OS == "macos" {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-darwin/ic-admin.gz")
        } else {
            format!("https://download.dfinity.systems/ic/{version}/binaries/x86_64-linux/ic-admin.gz")
        };
        info!("Downloading ic-admin version: {} from {}", version, url);
        let body = reqwest::get(url).await?.bytes().await?;
        let mut decoded = GzDecoder::new(body.as_ref());

        let path_parent = path.parent().expect("path parent unwrap failed!");
        std::fs::create_dir_all(path_parent)
            .unwrap_or_else(|_| panic!("create_dir_all failed for {}", path_parent.display()));
        let mut out = std::fs::File::create(path)?;
        std::io::copy(&mut decoded, &mut out)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("Using ic-admin: {}", path.display());

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
    use wiremock::MockServer;

    #[ignore]
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

        // Start a background HTTP server on a random local port
        let mock_server = MockServer::start().await;

        for cmd in test_cases {
            let cli = IcAdminWrapper {
                nns_url: url::Url::from_str(&mock_server.uri()).unwrap(),
                yes: false,
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
            let opts = ProposeOptions {
                title: Some("test-proposal".to_string()),
                summary: Some("test-summray".to_string()),
                ..Default::default()
            };

            let vector = vec![
                if !cli.yes {
                    vec!["--dry-run".to_string()]
                } else {
                    Default::default()
                },
                opts.title
                    .map(|t| vec!["--proposal-title".to_string(), t])
                    .unwrap_or_default(),
                opts.summary
                    .map(|s| {
                        vec![
                            "--summary".to_string(),
                            format!(
                                "{}{}",
                                s,
                                opts.motivation
                                    .map(|m| format!("\n\nMotivation: {m}"))
                                    .unwrap_or_default(),
                            ),
                        ]
                    })
                    .unwrap_or_default(),
                cli.neuron.as_ref().map(|n| n.as_arg_vec()).unwrap_or_default(),
                cmd.args(),
            ]
            .concat()
            .to_vec();
            let out = with_ic_admin(Default::default(), async {
                cli.run(&cmd.get_command_name(), &vector, true)
                    .map_err(|e| anyhow::anyhow!(e))
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
