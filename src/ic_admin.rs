use anyhow::{anyhow, Result};
use colored::Colorize;
use ic_base_types::PrincipalId;
use log::{debug, info, warn};
use python_input::input;
use std::process::Command;
use strum::Display;

use crate::cli::Opts;

#[derive(Clone)]
pub struct Cli {
    ic_admin: Option<String>,
    nns_url: Option<String>,
    dry_run: bool,

    hsm_pin: Option<String>,
    hsm_slot: Option<String>,
    hsm_key_id: Option<String>,
    neuron_id: Option<String>,
}

impl Cli {
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
        fn print_ic_admin_command_line(ic_admin_path: &String, ic_admin_args: &[String]) {
            println!(
                "$ {} {}",
                ic_admin_path.yellow(),
                shlex::join(ic_admin_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>()).yellow()
            );
        }

        if !self.dry_run {
            info!("Running the ic-admin command");
            print_ic_admin_command_line(ic_admin_path, &ic_admin_args);

            let output = Command::new(ic_admin_path).args(ic_admin_args).output()?;
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
            print_ic_admin_command_line(ic_admin_path, &ic_admin_args);

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
                    .map(|s| vec!["--proposer".to_string(), s])
                    .unwrap_or_default(),
                cmd.args(),
            ]
            .concat()
            .as_slice(),
        )
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
}

impl ProposeCommand {
    fn args(&self) -> Vec<String> {
        match &self {
            ProposeCommand::AddNodesToSubnet { subnet_id, nodes } => vec![
                nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
                vec!["--subnet-id".to_string(), subnet_id.to_string()],
            ]
            .concat(),
            ProposeCommand::RemoveNodesFromSubnet { nodes } => nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        }
    }
}

#[derive(Default)]
pub struct ProposeOptions {
    pub title: Option<String>,
    pub summary: Option<String>,
}

impl From<&Opts> for Cli {
    fn from(opts: &Opts) -> Self {
        Cli {
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
