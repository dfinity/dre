use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

use serde::Serialize;
use slog::{info, warn, Logger};
use tokio::{io::AsyncWriteExt, select};
use url::Url;

pub async fn download_loop(url: Url, logger: Logger, path: PathBuf, inputs: Vec<String>, rate: u64, transform_id: String) {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("Should be able to build a client");

    let mut interval = tokio::time::interval(Duration::from_secs(15));
    let mut current_hash = 0;
    loop {
        select! {
            tick = interval.tick() => {
                info!(logger, "Running loop @ {:?}", tick);
            },
            _ = tokio::signal::ctrl_c() => {
                info!(logger, "Received shutdown signal, exiting...");
                break
            }
        }

        let response = client.get(url.clone()).send().await;
        let response = match response {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                warn!(
                    logger,
                    "Received error status while downloading: {:?}\n{:?}",
                    r.status(),
                    r.text().await.expect("Should have text")
                );
                continue;
            }
            Err(e) => {
                warn!(logger, "Error while downloading: {:?}", e);
                continue;
            }
        };

        let response = match response.json::<BTreeMap<u32, String>>().await {
            Ok(r) => r,
            Err(e) => {
                warn!(logger, "Failed to parse response: {:?}", e);
                continue;
            }
        };

        let mut hasher = DefaultHasher::new();
        response.hash(&mut hasher);
        let new_hash = hasher.finish();
        if new_hash == current_hash {
            info!(logger, "Hash hasn't changed, skipping");
            continue;
        }

        info!(logger, "Hash changed: {} -> {}", current_hash, new_hash);
        current_hash = new_hash;

        let response = match response.is_empty() {
            true => "r'.*'".to_string(),
            false => response.values().map(|s| format!("r'{}'", s)).collect::<Vec<String>>().join(","),
        };

        let transform = VectorSampleTransform {
            _type: "sample".to_string(),
            inputs: inputs.clone(),
            key_field: "MESSAGE".to_string(),
            rate,
            exclude: format!("!match_any(.MESSAGE, [{}])", response),
        };

        let mut transforms = BTreeMap::new();
        transforms.insert(&transform_id, transform);
        let mut total = BTreeMap::new();
        total.insert("transforms", transforms);

        let transform = serde_json::to_string_pretty(&total).expect("Should be able to serialize");
        let mut file = tokio::fs::File::create(path.clone()).await.expect("Should be able to create file");
        file.write_all(transform.as_bytes()).await.expect("Should be able to write");
    }
}

#[derive(Debug, Serialize, Clone)]
struct VectorSampleTransform {
    #[serde(rename = "type")]
    _type: String,
    inputs: Vec<String>,
    key_field: String,
    rate: u64,
    exclude: String,
}
