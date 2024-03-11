use std::time::Duration;

use crate::calculation::should_proceed::should_proceed;
use chrono::{Local, NaiveDate};
use ic_management_backend::registry::RegistryState;
use prometheus_http_query::Client;
use serde::Deserialize;
use slog::{info, Logger};

use self::current_release_finder::find_latest_release;

mod current_release_finder;
mod should_proceed;

#[derive(Deserialize, Clone, Default)]
pub struct Index {
    pub rollout: Rollout,
    pub releases: Vec<Release>,
}

#[derive(Deserialize, Clone, Default)]
pub struct Rollout {
    #[serde(default)]
    pub pause: bool,
    pub skip_days: Vec<NaiveDate>,
    pub stages: Vec<Stage>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]

pub struct Stage {
    pub subnets: Vec<String>,
    #[serde(with = "humantime_serde")]
    pub bake_time: Duration,
    pub wait_for_next_week: bool,
    update_unassigned_nodes: bool,
}

#[derive(Deserialize, Clone, Default)]
pub struct Release {
    pub rc_name: String,
    pub versions: Vec<Version>,
}

#[derive(Deserialize, Clone, Default)]
pub struct Version {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub release_notes_read: bool,
    #[serde(default)]
    pub subnets: Vec<String>,
}

pub async fn calculate_progress(
    logger: &Logger,
    index: Index,
    _prometheus_client: &Client,
    _registry_state: RegistryState,
) -> anyhow::Result<Vec<String>> {
    if !should_proceed(&index, Local::now().to_utc().date_naive()) {
        info!(logger, "Rollout controller paused or should skip this day.");
        return Ok(vec![]);
    }

    let _latest_release = find_latest_release(&index)?;

    Ok(vec![])
}
