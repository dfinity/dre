use anyhow::anyhow;
use chrono::Utc;
use colored::Colorize;
use log::{debug, info, warn};
use python_input::input;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

/// Return the configuration value from the environment.
pub fn env_cfg(key: &str) -> String {
    match env::var(key) {
        Ok(value) => value,
        Err(err) => panic!(
            "Environment variable `{}` is not set. Please update the .env file. {}",
            key, err
        ),
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ProposalStatus {
    pub(crate) id: u32,
    pub(crate) summary: String,
    pub(crate) timestamp_seconds: u64,
    pub(crate) executed_timestamp_seconds: u64,
    pub(crate) failed_timestamp_seconds: u64,
    pub(crate) failure_reason: String,
}

/// Get status of an NNS proposal
pub fn get_proposal_status(proposal_id: i32) -> Result<String, anyhow::Error> {
    let mut dfx_args =
        shlex::split("--identity default canister --no-wallet --network=mercury call governance get_proposal_info")
            .expect("shlex split failed");
    debug!("get_proposal_status {}", proposal_id);
    dfx_args.push(format!("{}", proposal_id));
    let output = Command::new(env_cfg("DFX"))
        .args(dfx_args)
        .current_dir("canisters")
        .output()?;

    let stdout = String::from_utf8_lossy(output.stdout.as_ref()).to_string();
    println!("Proposal parsed: {:?}", proposal_text_parse(&stdout));
    info!("STDOUT:\n{}", stdout);
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(output.stderr.as_ref()).to_string();
        warn!("STDERR:\n{}", stderr);
    }
    Ok(stdout)
}

fn proposal_text_parse(text: &str) -> Result<ProposalStatus, anyhow::Error> {
    Ok(ProposalStatus {
        id: regex_find(r"(?m)^\s*id = opt record \{ id = ([\d_]+) \};$", text)?
            .replace("_", "")
            .parse::<u32>()?,
        summary: regex_find(r#"(?m)^\s*      summary = "(.*)";$"#, text)?,
        timestamp_seconds: regex_find(r"(?m)^\s*proposal_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<u64>()?,
        executed_timestamp_seconds: regex_find(r"(?m)^\s*executed_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<u64>()?,
        failed_timestamp_seconds: regex_find(r"(?m)^\s*failed_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<u64>()?,
        failure_reason: regex_find(r#"(?ms)^\s*failure_reason = (null|opt record \{.+?\});$"#, text)?,
    })
}

fn regex_find(needle: &str, haystack: &str) -> Result<String, anyhow::Error> {
    let re = Regex::new(needle).unwrap();
    if let Some(cap) = re.captures_iter(haystack).next() {
        return Ok(cap[1].parse::<String>().unwrap());
    }
    Err(anyhow!(
        "Search string '{}' could not be found in the output string:\n{}",
        needle,
        haystack,
    ))
}

pub(crate) fn ic_admin_run(args: &[String], confirmed: bool) -> Result<String, anyhow::Error> {
    let args_basic = vec![
        "--use-hsm".to_string(),
        "--slot".to_string(),
        env_cfg("hsm_slot"),
        "--key-id".to_string(),
        env_cfg("hsm_key_id"),
        "--pin".to_string(),
        env_cfg("hsm_pin"),
        "--nns-url".to_string(),
        env_cfg("nns_url"),
    ];

    let ic_admin_args = [args_basic, args.to_owned()].concat();

    fn print_ic_admin_command_line(ic_admin_args: &[String]) {
        println!(
            "$ {} {}",
            env_cfg("IC_ADMIN").yellow(),
            shlex::join(ic_admin_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>()).yellow()
        );
    }

    if confirmed {
        info!("Running the ic-admin command");
        print_ic_admin_command_line(&ic_admin_args);

        let output = Command::new(env_cfg("IC_ADMIN")).args(ic_admin_args).output()?;
        let stdout = String::from_utf8_lossy(output.stdout.as_ref()).to_string();
        info!("STDOUT:\n{}", stdout);
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(output.stderr.as_ref()).to_string();
            warn!("STDERR:\n{}", stderr);
        }
        Ok(stdout)
    } else {
        println!("Please confirm enqueueing the following ic-admin command");
        // Show the user the line that would be executed and let them decide if they
        // want to proceed.
        print_ic_admin_command_line(&ic_admin_args);

        let buffer = input("Would you like to proceed [y/N]? ");
        match buffer.to_uppercase().as_str() {
            "Y" | "YES" => Ok("User confirmed".to_string()),
            _ => Err(anyhow!("Cancelling operation, user entered '{}'", buffer.as_str(),)),
        }
    }
}

pub fn nns_proposals_repo_new_subnet_management(
    summary_long: &str,
    summary_short: &str,
) -> Result<String, anyhow::Error> {
    let repo_path = std::path::PathBuf::from(env_cfg("NNS_PROPOSALS_REPO_PATH"));

    let status = Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", "main"])
        .status()
        .expect("failed to execute process");
    if !status.success() {
        panic!("Command execution failed. Bailing out.")
    }

    let status = Command::new("git")
        .current_dir(&repo_path)
        .args(["pull", "--rebase"])
        .status()
        .expect("failed to execute process");
    if !status.success() {
        panic!("Command execution failed. Bailing out.")
    }

    let date_time = Utc::today().format("%Y-%m-%dT%H_%M_%SZ.md");
    let file_subdir = format!("proposals/subnet_management/{}", date_time);

    info!("Creating a new file: {}", file_subdir);
    let mut file = File::create(repo_path.join(&file_subdir))?;
    file.write_all(summary_long.as_bytes())?;

    let status = Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "--message", summary_short, &file_subdir])
        .status()
        .expect("failed to execute process");
    if !status.success() {
        panic!("Command execution failed. Bailing out.")
    }

    Ok(format!(
        "https://github.com/ic-association/nns-proposals/blob/main/{}",
        &file_subdir
    ))
}

#[cfg(test)]
mod tests;
