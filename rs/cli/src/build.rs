// Construct a useful and specific version string for the CLI
use std::{collections::BTreeMap, path::PathBuf, process::Command, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::json;

fn main() {
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    let git_rev = option_env!("GIT_REV").map(String::from).unwrap_or_else(|| {
        String::from_utf8(
            // https://stackoverflow.com/questions/21017300/git-command-to-get-head-sha1-with-dirty-suffix-if-workspace-is-not-clean
            Command::new("git").args(["describe", "--always", "--dirty"]).output().unwrap().stdout,
        )
        .unwrap()
    });
    if !git_rev.is_empty() {
        println!(
            "cargo:rustc-env=CARGO_PKG_VERSION={}",
            option_env!("CARGO_PKG_VERSION").map(|v| format!("{}-{}", v, git_rev)).unwrap_or_default()
        );
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path_to_non_public_subnets =
        std::fs::canonicalize(option_env!("NON_PUBLIC_SUBNETS").unwrap_or("../../facts-db/non_public_subnets.csv")).unwrap();

    std::fs::copy(&path_to_non_public_subnets, format!("{}/non_public_subnets.csv", out_dir))
        .unwrap_or_else(|e| panic!("Error with file {}: {:?}", path_to_non_public_subnets.display(), e));

    maybe_fetch_and_update_subnet_topics()
}

fn maybe_fetch_and_update_subnet_topics() {
    // Check if generated json exists

    let subnet_topics_file_path: PathBuf = PathBuf::from_str(&format!("{}/src/assets/subnet_topic_map.json", env!("CARGO_MANIFEST_DIR"))).unwrap();

    if subnet_topics_file_path.exists() && option_env!("GENERATE_SUBNET_TOPICS_FILE").is_none() {
        // File exists and user didn't specify recreating the file
        // Skip to save time
        return;
    }

    let subnets: serde_json::Value = match reqwest::blocking::get("https://ic-api.internetcomputer.org/api/v3/subnets?format=json") {
        Ok(response) => response.json().unwrap(),
        r => {
            println!("cargo:warning=Failed to fetch subnets from public dashboard. Response: {:?}", r);
            json!({
                "subnets": []
            })
        }
    };

    let subnets = match subnets.get("subnets").unwrap().as_array() {
        Some(subnets) => subnets,
        None => unreachable!("Shouldn't happen because of previous block"),
    };

    // If there is something already in the file,
    // fetch that just so we don't override everything
    let existing_sunbets_topics: BTreeMap<String, FoundPost> =
        serde_json::from_str(&std::fs::read_to_string(&subnet_topics_file_path).unwrap_or("{}".to_string())).unwrap();

    let new_subnet_topic_map: BTreeMap<String, FoundPost> = subnets
        .iter()
        .map(|subnet| {
            let subnet_id = subnet.get("subnet_id").map(|val| val.as_str().unwrap()).unwrap();
            (subnet_id.to_string(), FoundPost::default())
        })
        .filter(|(key, _)| !existing_sunbets_topics.contains_key(key))
        .collect();

    let chained = existing_sunbets_topics
        .into_iter()
        .chain(new_subnet_topic_map.into_iter())
        .collect::<BTreeMap<String, FoundPost>>();

    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    chained.serialize(&mut ser).unwrap();

    std::fs::write(subnet_topics_file_path, buf).unwrap();
}

#[derive(Default, Serialize, Deserialize)]
struct FoundPost {
    topic_id: u64,
    slug: String,
}
