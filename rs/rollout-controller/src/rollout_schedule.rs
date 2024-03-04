use std::path::PathBuf;

use slog::Logger;

pub async fn calculate_progress(logger: &Logger, release_index: &PathBuf) -> anyhow::Result<()> {
    // Deserialize the index

    // Check if the desired rollout version is elected

    // Iterate over stages and compare desired state from file and the state it is live
    // Take into consideration:
    //      0. Check if the plan is paused. If it is do nothing
    //      1. If the subnet is on the desired version, proceed
    //      2. If the subnet isn't on the desired version:
    //          2.1. Check if there is an open proposal for upgrading, if there isn't place one
    //          2.2. If the proposal is executed check when it was upgraded and query prometheus
    //               to see if there were any alerts and if the bake time has passed. If the bake
    //               time didn't pass don't do anything. If there were alerts don't do anything.
}

pub struct Index {
    pub rollout: Rollout,
    pub features: Vec<Feature>,
    pub releases: Vec<Release>,
}

pub struct Rollout {
    pub rc_name: String,
    pub pause: bool,
    pub skip_days: Vec<String>,
    pub stages: Vec<Stage>,
}

pub struct Stage {
    pub subnets: Vec<String>,
    pub bake_time: String,
    pub wait_for_next_week: bool,
}

pub struct Feature {
    pub name: String,
    pub subnets: Vec<String>,
}

pub struct Release {
    pub name: String,
    pub version: String,
    pub publish: String,
    pub features: Vec<String>,
}
