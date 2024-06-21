use crate::registry::local_cache_path;
use ic_management_types::Network;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::{fs::File, io::AsyncWriteExt};

const IC_DASHBOARD_API: &str = "https://ic-api.internetcomputer.org/api";
const IC_API_REFRESH_INTERVAL_SECONDS: u64 = 60 * 60; // 1h

pub async fn query_ic_dashboard_list<T: DeserializeOwned>(network: &Network, query_what: &str) -> anyhow::Result<T> {
    let local_cache_file_path = local_cache_path()
        .join(PathBuf::from(query_what).file_name().unwrap())
        .with_extension("json");
    // if there is a local cache, use it if either:
    // - file was last updated before IC_API_REFRESH_INTERVAL_SECONDS, or
    // - we target a network other than the mainnet: IC dashboard has data only for the mainnet, so data does not matter
    let data = if local_cache_file_path.exists()
        && (!network.is_mainnet()
            || local_cache_file_path.metadata().unwrap().modified().unwrap().elapsed().unwrap().as_secs() < IC_API_REFRESH_INTERVAL_SECONDS)
    {
        let file = File::open(&local_cache_file_path).await?;
        let mut buf = Vec::new();
        BufReader::new(file).read_to_end(&mut buf).await?;
        buf
    } else {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/{}", IC_DASHBOARD_API, &query_what))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| anyhow::format_err!("failed to fetch response: {}", e))?
            .bytes()
            .await?
            .to_vec()
    };
    match serde_json::from_slice(data.as_slice()) {
        Ok(result) => {
            let mut file = File::create(&local_cache_file_path).await?;
            file.write_all(&data).await?;
            Ok(result)
        }
        Err(e) => Err(anyhow::format_err!("failed to parse as json: {}", e)),
    }
}
