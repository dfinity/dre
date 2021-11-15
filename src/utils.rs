use anyhow::anyhow;
use colored::Colorize;
use log::{debug, info, warn};
use python_input::input;
use regex::Regex;
use std::env;
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
pub struct ProposalStatus {
    pub id: u32,
    pub summary: String,
    pub timestamp_seconds: i64,
    pub executed_timestamp_seconds: i64,
    pub failed_timestamp_seconds: i64,
    pub failure_reason: String,
}

/// Get status of an NNS proposal
pub fn get_proposal_status(proposal_id: i32) -> Result<ProposalStatus, anyhow::Error> {
    debug!("get_proposal_status: {}", proposal_id);
    let mut dfx_args =
        shlex::split("--identity default canister --no-wallet --network=mercury call governance get_proposal_info")
            .expect("shlex split failed");
    dfx_args.push(format!("{}", proposal_id));

    fn log_dfx_command_line(dfx_args: &[String]) {
        debug!(
            "$ {} {}",
            env_cfg("DFX").yellow(),
            shlex::join(dfx_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>()).yellow()
        );
    }

    log_dfx_command_line(&dfx_args);

    let output = Command::new(env_cfg("DFX"))
        .args(dfx_args)
        .current_dir("canisters")
        .output()?;

    let stdout = String::from_utf8_lossy(output.stdout.as_ref()).to_string();
    debug!("dfx STDOUT:\n{}", stdout);
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(output.stderr.as_ref()).to_string();
        warn!("dfx STDERR:\n{}", stderr);
    }
    let result = proposal_text_parse(&stdout)?;
    info!("Parsed proposal status: {:?}", result);
    Ok(result)
}

fn proposal_text_parse(text: &str) -> Result<ProposalStatus, anyhow::Error> {
    debug!("Parsing proposal text: {:?}", text);
    Ok(ProposalStatus {
        id: regex_find(r"(?m)^\s*id = opt record \{ id = ([\d_]+) \};$", text)?
            .replace("_", "")
            .parse::<u32>()?,
        summary: regex_find(r#"(?ms)^\s*      summary = "(.*)";$"#, text)?,
        timestamp_seconds: regex_find(r"(?m)^\s*proposal_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<i64>()?,
        executed_timestamp_seconds: regex_find(r"(?m)^\s*executed_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<i64>()?,
        failed_timestamp_seconds: regex_find(r"(?m)^\s*failed_timestamp_seconds = ([\d_]+);$", text)?
            .replace("_", "")
            .parse::<i64>()?,
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
        print_ic_admin_command_line(&ic_admin_args);

        let buffer = input("Would you like to proceed [y/N]? ");
        match buffer.to_uppercase().as_str() {
            "Y" | "YES" => Ok("User confirmed".to_string()),
            _ => Err(anyhow!("Cancelling operation, user entered '{}'", buffer.as_str(),)),
        }
    }
}

#[cfg(test)]
mod tests;
