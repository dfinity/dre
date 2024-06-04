use crate::parsed_cli::UpdateVersion;
use anyhow::{anyhow, Error, Result};
use colored::Colorize;
use dialoguer::Confirm;
use flate2::read::GzDecoder;
use futures::stream::{self, StreamExt};
use futures::Future;
use ic_base_types::PrincipalId;
use ic_interfaces_registry::RegistryClient;
use ic_management_backend::registry::{local_registry_path, RegistryFamilyEntries, RegistryState};
use ic_management_types::{Artifact, Network};
use ic_protobuf::registry::firewall::v1::{FirewallRule, FirewallRuleSet};
use ic_protobuf::registry::subnet::v1::SubnetRecord;
use ic_registry_keys::{make_firewall_rules_record_key, FirewallRulesScope};
use ic_registry_local_registry::LocalRegistry;
use itertools::Itertools;
use log::{error, info, warn};
use prost::Message;
use regex::Regex;
use reqwest::StatusCode;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
use std::time::Duration;
use std::{fmt::Display, path::Path, process::Command};
use strum::Display;
use tempfile::NamedTempFile;

use crate::defaults;
use crate::detect_neuron::{Auth, Neuron};
use crate::parsed_cli::ParsedCli;

const MAX_SUMMARY_CHAR_COUNT: usize = 29000;

#[derive(Clone, Serialize, PartialEq)]
enum FirewallRuleModificationType {
    Addition,
    Update,
    Removal,
}

impl Display for FirewallRuleModificationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Addition => write!(f, "add"),
            Self::Update => write!(f, "update"),
            Self::Removal => write!(f, "remove"),
        }
    }
}

#[derive(Clone, Serialize)]
struct FirewallRuleModification {
    change_type: FirewallRuleModificationType,
    rule_being_modified: FirewallRule,
    position: usize,
}

impl FirewallRuleModification {
    fn addition(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Addition,
            rule_being_modified: rule,
            position,
        }
    }
    fn update(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Update,
            rule_being_modified: rule,
            position,
        }
    }
    fn removal(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Removal,
            rule_being_modified: rule,
            position,
        }
    }
}

struct FirewallRuleModifications {
    raw: Vec<FirewallRuleModification>,
}

impl FirewallRuleModifications {
    fn new() -> Self {
        FirewallRuleModifications { raw: vec![] }
    }

    fn addition(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::addition(position, rule))
    }

    fn update(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::update(position, rule))
    }

    fn removal(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::removal(position, rule))
    }

    fn reverse_sorted(&self) -> Vec<FirewallRuleModification> {
        let mut sorted = self.raw.to_vec();
        sorted.sort_by(|first, second| first.position.partial_cmp(&second.position).unwrap());
        sorted.reverse();
        sorted
    }

    fn reverse_sorted_and_batched(&self) -> Vec<(FirewallRuleModificationType, Vec<FirewallRuleModification>)> {
        let mut batches: Vec<(FirewallRuleModificationType, Vec<FirewallRuleModification>)> = vec![];
        let mut current_batch: Vec<FirewallRuleModification> = vec![];
        let mut modtype: Option<FirewallRuleModificationType> = None;
        for modif in self.reverse_sorted().iter() {
            if modtype.is_none() {
                modtype = Some(modif.clone().change_type);
            }
            if modtype.clone().unwrap() == modif.change_type {
                current_batch.push(modif.clone())
            } else {
                batches.push((current_batch[0].clone().change_type, current_batch));
                current_batch = vec![];
                modtype = Some(modif.clone().change_type);
            }
        }
        if !current_batch.is_empty() {
            batches.push((current_batch[0].clone().change_type, current_batch))
        }
        batches
    }
}

#[derive(Clone)]
pub struct IcAdminWrapper {
    network: Network,
    ic_admin_bin_path: Option<String>,
    proceed_without_confirmation: bool,
    neuron: Neuron,
}

impl IcAdminWrapper {
    pub fn new(network: Network, ic_admin_bin_path: Option<String>, proceed_without_confirmation: bool, neuron: Neuron) -> Self {
        Self {
            network,
            ic_admin_bin_path,
            proceed_without_confirmation,
            neuron,
        }
    }

    pub fn as_automation(self) -> Self {
        Self {
            network: self.network,
            ic_admin_bin_path: self.ic_admin_bin_path,
            proceed_without_confirmation: self.proceed_without_confirmation,
            neuron: self.neuron.as_automation(),
        }
    }

    pub fn from_cli(cli: ParsedCli) -> Self {
        Self {
            network: cli.network,
            ic_admin_bin_path: cli.ic_admin_bin_path,
            proceed_without_confirmation: cli.yes,
            neuron: cli.neuron,
        }
    }

    async fn print_ic_admin_command_line(&self, cmd: &Command) {
        let auth = self.neuron.get_auth().await.unwrap();
        info!(
            "running ic-admin: \n$ {}{}",
            cmd.get_program().to_str().unwrap().yellow(),
            cmd.get_args()
                .map(|s| s.to_str().unwrap().to_string())
                .fold("".to_string(), |acc, s| {
                    let s = if s.contains('\n') { format!(r#""{}""#, s) } else { s };
                    let hsm_pin = if let Auth::Hsm { pin, .. } = &auth { pin } else { "" };
                    if hsm_pin == s {
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

    async fn _exec(&self, cmd: ProposeCommand, opts: ProposeOptions, as_simulation: bool) -> anyhow::Result<String> {
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

        self.run(
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
                            format!("{}{}", s, opts.motivation.map(|m| format!("\n\nMotivation: {m}")).unwrap_or_default(),),
                        ]
                    })
                    .unwrap_or_default(),
                self.neuron.as_arg_vec(true).await?,
                cmd.args(),
            ]
            .concat()
            .as_slice(),
            true,
            false,
        )
        .await
    }

    pub async fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions, simulate: bool) -> anyhow::Result<String> {
        // Simulated, or --help executions run immediately and do not proceed.
        if simulate || cmd.args().contains(&String::from("--help")) || cmd.args().contains(&String::from("--dry-run")) {
            return self._exec(cmd, opts, simulate).await;
        }

        // If --yes was not specified, ask the user if they want to proceed
        if !self.proceed_without_confirmation {
            self._exec(cmd.clone(), opts.clone(), true).await?;
        }

        if self.proceed_without_confirmation || Confirm::new().with_prompt("Do you want to continue?").default(false).interact()? {
            // User confirmed the desire to submit the proposal and no obvious problems were
            // found. Proceeding!
            self._exec(cmd, opts, false).await
        } else {
            Err(anyhow::anyhow!("Action aborted"))
        }
    }

    async fn _run_ic_admin_with_args(&self, ic_admin_args: &[String], with_auth: bool, silent: bool) -> anyhow::Result<String> {
        let ic_admin_path = self.ic_admin_bin_path.clone().unwrap_or_else(|| "ic-admin".to_string());
        let mut cmd = Command::new(ic_admin_path);
        let auth_options = if with_auth { self.neuron.get_auth().await?.as_arg_vec() } else { vec![] };
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

    pub async fn run(&self, command: &str, args: &[String], with_auth: bool, silent: bool) -> anyhow::Result<String> {
        let ic_admin_args = [&[command.to_string()], args].concat();
        self._run_ic_admin_with_args(&ic_admin_args, with_auth, silent).await
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

    /// Run an `ic-admin get-*` command directly, and without an HSM
    pub async fn run_passthrough_get(&self, args: &[String], silent: bool) -> anyhow::Result<String> {
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

        let stdout = self
            .run(&args[0], &args.iter().skip(1).cloned().collect::<Vec<_>>(), false, silent)
            .await?;
        Ok(stdout)
    }

    /// Run an `ic-admin propose-to-*` command directly
    pub async fn run_passthrough_propose(&self, args: &[String], simulate: bool) -> anyhow::Result<()> {
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
        let simulate = simulate || cmd.args().contains(&String::from("--dry-run"));
        self.propose_run(cmd, Default::default(), simulate).await?;
        Ok(())
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
        let subdir = format!("{}{}", url.domain().expect("url.domain() is None"), url.path().to_owned());
        // replace special characters in subdir with _
        let subdir = subdir.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        let download_dir = format!("{}/tmp/ic/{}", dirs::home_dir().expect("home_dir is not set").as_path().display(), subdir);
        let download_dir = Path::new(&download_dir);

        std::fs::create_dir_all(download_dir).unwrap_or_else(|_| panic!("create_dir_all failed for {}", download_dir.display()));

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
        let stringified_hash = hash[..].iter().map(|byte| format!("{:01$x?}", byte, 2)).collect::<Vec<String>>().join("");
        info!("File saved at {} has sha256 {}", download_image.display(), stringified_hash);
        Ok(stringified_hash)
    }

    async fn download_images_and_validate_sha256(
        image: &Artifact,
        version: &String,
        ignore_missing_urls: bool,
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
        let hashes_unique = hash_and_valid_urls.iter().map(|(h, _)| h.clone()).unique().collect::<Vec<String>>();
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
                    hash_and_valid_urls.iter().map(|(h, u)| format!("{}  {}", h, u)).join("\n")
                ))
            }
        };
        let update_urls = hash_and_valid_urls.into_iter().map(|(_, u)| u.clone()).collect::<Vec<String>>();

        if update_urls.is_empty() {
            return Err(anyhow::anyhow!(
                "Unable to download the update image from none of the following URLs: {}",
                update_urls.join(", ")
            ));
        } else if update_urls.len() == 1 {
            if ignore_missing_urls {
                warn!("Only 1 update image is available. At least 2 should be present in the proposal");
            } else {
                return Err(anyhow::anyhow!(
                    "Only 1 update image is available. At least 2 should be present in the proposal"
                ));
            }
        }
        Ok((update_urls, expected_hash))
    }

    pub async fn prepare_to_propose_to_revise_elected_versions(
        release_artifact: &Artifact,
        version: &String,
        release_tag: &String,
        force: bool,
        retire_versions: Option<Vec<String>>,
    ) -> anyhow::Result<UpdateVersion> {
        let (update_urls, expected_hash) = Self::download_images_and_validate_sha256(release_artifact, version, force).await?;

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

        // Remove <!--...--> from the commit
        // Leading or trailing spaces are removed as well and replaced with a single space.
        // Regex can be analyzed and tested at:
        // https://rregex.dev/?version=1.7&method=replace&regex=%5Cs*%3C%21--.%2B%3F--%3E%5Cs*&replace=+&text=*+%5Babc%5D+%3C%21--+ignored+1+--%3E+line%0A*+%5Babc%5D+%3C%21--+ignored+2+--%3E+comment+1+%3C%21--+ignored+3+--%3E+comment+2%0A
        let re_comment = Regex::new(r"\s*<!--.+?-->\s*").unwrap();
        let mut builder = edit::Builder::new();
        let with_suffix = builder.suffix(".md");
        let edited = edit::edit_with_builder(template, with_suffix)?
            .trim()
            .replace("\r(\n)?", "\n")
            .split('\n')
            .map(|f| {
                let f = re_comment.replace_all(f.trim(), " ");

                if !f.starts_with('*') {
                    return f.to_string();
                }
                match f.split_once(']') {
                    Some((left, message)) => {
                        let commit_hash = left.split_once('[').unwrap().1.to_string();

                        format!("* [[{}](https://github.com/dfinity/ic/commit/{})] {}", commit_hash, commit_hash, message)
                    }
                    None => f.to_string(),
                }
            })
            .join("\n");
        if edited.contains(&String::from("Remove this block of text from the proposal.")) {
            Err(anyhow::anyhow!("The edited proposal text has not been edited to add release notes."))
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
                None => format!("Elect new IC/{} revision (commit {})", release_artifact.capitalized(), &version[..8]),
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

    pub async fn update_unassigned_nodes(&self, nns_subned_id: &String, network: &Network, simulate: bool) -> Result<(), Error> {
        let local_registry_path = local_registry_path(network);
        let local_registry = LocalRegistry::new(local_registry_path, Duration::from_secs(10))
            .map_err(|e| anyhow::anyhow!("Error in creating local registry instance: {:?}", e))?;

        local_registry
            .sync_with_nns()
            .await
            .map_err(|e| anyhow::anyhow!("Error when syncing with NNS: {:?}", e))?;

        let subnets = local_registry.get_family_entries::<SubnetRecord>()?;

        let nns = match subnets.get_key_value(nns_subned_id) {
            Some((_, value)) => value,
            None => return Err(anyhow::anyhow!("Couldn't find nns subnet with id '{}'", nns_subned_id)),
        };

        let registry_state = RegistryState::new(network, true).await;
        let unassigned_version = registry_state.get_unassigned_nodes_replica_version().await?;

        if nns.replica_version_id.eq(&unassigned_version) {
            info!(
                "Unassigned nodes and nns are of the same version '{}', skipping proposal submition.",
                unassigned_version
            );
            return Ok(());
        }

        info!(
            "NNS version '{}' and Unassigned nodes '{}' differ",
            nns.replica_version_id, unassigned_version
        );

        let command = ProposeCommand::DeployGuestosToAllUnassignedNodes {
            replica_version: nns.replica_version_id.clone(),
        };
        let options = ProposeOptions {
            summary: Some("Update the unassigned nodes to the latest rolled-out version".to_string()),
            motivation: None,
            title: Some("Update all unassigned nodes".to_string()),
        };

        self.propose_run(command, options, simulate).await?;
        Ok(())
    }

    pub async fn update_firewall(
        &self,
        network: &Network,
        propose_options: ProposeOptions,
        firewall_rules_scope: FirewallRulesScope,
        simulate: bool,
    ) -> Result<(), Error> {
        let local_registry_path = local_registry_path(network);
        let local_registry = LocalRegistry::new(local_registry_path, Duration::from_secs(10))
            .map_err(|e| anyhow::anyhow!("Error in creating local registry instance: {:?}", e))?;

        local_registry
            .sync_with_nns()
            .await
            .map_err(|e| anyhow::anyhow!("Error when syncing with NNS: {:?}", e))?;

        let value = local_registry
            .get_value(
                &make_firewall_rules_record_key(&firewall_rules_scope),
                local_registry.get_latest_version(),
            )
            .map_err(|e| anyhow::anyhow!("Error fetching firewall rules for replica nodes: {:?}", e))?;

        let rules = if let Some(value) = value {
            FirewallRuleSet::decode(value.as_slice()).map_err(|e| anyhow::anyhow!("Failed to deserialize firewall ruleset: {:?}", e))?
        } else {
            FirewallRuleSet::default()
        };

        let rules: BTreeMap<usize, &FirewallRule> = rules.entries.iter().enumerate().sorted_by(|a, b| a.0.cmp(&b.0)).collect();

        let mut builder = edit::Builder::new();
        let with_suffix = builder.suffix(".json");
        let pretty = serde_json::to_string_pretty(&rules).map_err(|e| anyhow::anyhow!("Error serializing ruleset to string: {:?}", e))?;
        let edited: BTreeMap<usize, FirewallRule>;
        loop {
            info!("Spawning edit window...");
            let edited_string = edit::edit_with_builder(pretty.clone(), with_suffix)?;
            match serde_json::from_str(&edited_string) {
                Ok(ruleset) => {
                    edited = ruleset;
                    break;
                }
                Err(e) => {
                    warn!("Couldn't parse the input you provided, please retry. Error: {:?}", e);
                }
            }
        }

        let mut added_entries: BTreeMap<usize, &FirewallRule> = BTreeMap::new();
        let mut updated_entries: BTreeMap<usize, &FirewallRule> = BTreeMap::new();
        for (key, rule) in edited.iter() {
            if let Some(old_rule) = rules.get(key) {
                if rule != *old_rule {
                    // Same key but different value meaning it was just updated
                    updated_entries.insert(*key, rule);
                }
                continue;
            }
            // Doesn't exist in old ones meaning it was just added
            added_entries.insert(*key, rule);
        }

        // Collect removed entries (keys from old set not present in new set)
        let removed_entries: BTreeMap<usize, &FirewallRule> = rules.into_iter().filter(|(key, _)| !edited.contains_key(key)).collect();

        let mut mods = FirewallRuleModifications::new();
        for (pos, rule) in added_entries.into_iter() {
            mods.addition(pos, rule.clone());
        }
        for (pos, rule) in updated_entries.into_iter() {
            mods.update(pos, rule.clone());
        }
        for (pos, rule) in removed_entries.into_iter() {
            mods.removal(pos, rule.clone());
        }

        let reverse_sorted = mods.reverse_sorted_and_batched();
        if reverse_sorted.is_empty() {
            info!("No modifications should be made");
            return Ok(());
        }
        let diff = serde_json::to_string_pretty(&reverse_sorted).unwrap();
        info!("Pretty printing diff:\n{}", diff);
        /*if reverse_sorted.len() > 1 {
            return Err(anyhow::anyhow!(
                "Cannot currently apply more than 1 change at a time due to hash changes"
            ));
        }*/

        //TODO: adapt to use set-firewall config so we can modify more than 1 rule at a time

        async fn submit_proposal(
            admin_wrapper: &IcAdminWrapper,
            modifications: Vec<FirewallRuleModification>,
            propose_options: ProposeOptions,
            firewall_rules_scope: FirewallRulesScope,
            simulate: bool,
        ) -> anyhow::Result<()> {
            let positions = modifications.iter().map(|modif| modif.position).join(",");
            let change_type = modifications[0].clone().change_type;

            let mut file = NamedTempFile::new().map_err(|e| anyhow::anyhow!("Couldn't create temp file: {:?}", e))?;

            let test_args = match change_type {
                FirewallRuleModificationType::Removal => vec![
                    "--test".to_string(),
                    firewall_rules_scope.to_string(),
                    positions.to_string(),
                    "none".to_string(),
                ],
                _ => {
                    let rules = modifications.iter().map(|modif| modif.clone().rule_being_modified).collect::<Vec<_>>();
                    let serialized = serde_json::to_string(&rules).unwrap();
                    file.write_all(serialized.as_bytes())
                        .map_err(|e| anyhow::anyhow!("Couldn't write to tempfile: {:?}", e))?;
                    vec![
                        "--test".to_string(),
                        firewall_rules_scope.to_string(),
                        file.path().to_str().unwrap().to_string(),
                        positions.to_string(),
                        "none".to_string(),
                    ]
                }
            };

            let cmd = ProposeCommand::Raw {
                command: format!("{}-firewall-rules", change_type),
                args: test_args.clone(),
            };

            let output = admin_wrapper
                .propose_run(cmd, propose_options.clone(), true)
                .await
                .map_err(|e| anyhow::anyhow!("Couldn't execute test for {}-firewall-rules: {:?}", change_type, e))?;

            let parsed: serde_json::Value = serde_json::from_str(&output)
                .map_err(|e| anyhow::anyhow!("Error deserializing --test output while performing '{}': {:?}", change_type, e))?;
            let hash = match parsed.get("hash") {
                Some(serde_json::Value::String(hash)) => hash,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Couldn't find string value for key 'hash'. Whole dump:\n{}",
                        serde_json::to_string_pretty(&parsed).unwrap()
                    ))
                }
            };
            info!("Computed hash for firewall rule at position '{}': {}", positions, hash);

            let mut final_args = test_args.clone();
            // Remove --test from head of args.
            let _ = final_args.remove(0);
            // Add the real hash to args.
            let last = final_args.last_mut().unwrap();
            *last = hash.to_string();

            let cmd = ProposeCommand::Raw {
                command: format!("{}-firewall-rules", change_type),
                args: final_args,
            };

            admin_wrapper.propose_run(cmd, propose_options.clone(), simulate).await?;

            Ok(())
        }

        // no more than one rule mod implemented currenty -- FIXME
        match reverse_sorted.into_iter().last() {
            Some((_, mods)) => submit_proposal(self, mods, propose_options.clone(), simulate).await,
            None => Err(anyhow::anyhow!("Expected to have one item for firewall rule modification")),
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
            Self::CreateSubnet { node_ids, replica_version } => {
                let mut args = vec!["--subnet-type".to_string(), "application".to_string()];

                args.push("--replica-version-id".to_string());
                args.push(replica_version.to_string());

                for id in node_ids {
                    args.push(id.to_string())
                }
                args
            }
            Self::DeployGuestosToAllUnassignedNodes { replica_version } => {
                vec!["--replica-version-id".to_string(), replica_version.clone()]
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
        std::fs::create_dir_all(path_parent).unwrap_or_else(|_| panic!("create_dir_all failed for {}", path_parent.display()));
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
    std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()));

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
            ProposeCommand::DeployGuestosToAllSubnetNodes {
                subnet: Default::default(),
                version: "0000000000000000000000000000000000000000".to_string(),
            },
        ];

        // Start a background HTTP server on a random local port
        let mock_server = MockServer::start().await;
        let network = Network::new("testnet", &vec![url::Url::from_str(&mock_server.uri()).unwrap()])
            .await
            .expect("Failed to create network");

        for cmd in test_cases {
            let cli = IcAdminWrapper {
                network: network.clone(),
                proceed_without_confirmation: false,
                neuron: Neuron::new(&network, Some(3), Some(file.path().to_string_lossy().to_string()), None, None, None).await,
                ic_admin_bin_path: None,
            };

            let cmd_name = cmd.to_string();
            let opts = ProposeOptions {
                title: Some("test-proposal".to_string()),
                summary: Some("test-summray".to_string()),
                ..Default::default()
            };

            let vector = [
                if !cli.proceed_without_confirmation {
                    vec!["--dry-run".to_string()]
                } else {
                    Default::default()
                },
                opts.title.map(|t| vec!["--proposal-title".to_string(), t]).unwrap_or_default(),
                opts.summary
                    .map(|s| {
                        vec![
                            "--summary".to_string(),
                            format!("{}{}", s, opts.motivation.map(|m| format!("\n\nMotivation: {m}")).unwrap_or_default(),),
                        ]
                    })
                    .unwrap_or_default(),
                cli.neuron.get_auth().await?.as_arg_vec(),
                cmd.args(),
            ]
            .concat()
            .to_vec();
            let out = with_ic_admin(Default::default(), async {
                cli.run(&cmd.get_command_name(), &vector, true, false)
                    .await
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
