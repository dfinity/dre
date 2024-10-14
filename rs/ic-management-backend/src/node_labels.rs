use std::path::PathBuf;

use ic_management_types::Guest;
use log::warn;

use crate::registry::DFINITY_DCS;

pub async fn query_guests(network: &String, local_cache: Option<PathBuf>) -> anyhow::Result<Vec<Guest>> {
    match (fetch_data(network).await, local_cache.as_ref()) {
        (Ok(data), Some(cache)) => {
            // Persist in a temp new location in case the
            // downloaded file is corrupt yaml
            let new_cache = cache.parent().unwrap().join("new_labels.yaml");
            if let Err(e) = std::fs::write(&new_cache, &data) {
                warn!("Failed writing temp new data to `{}` due to: {:?}", new_cache.display(), e);
            }

            parse_data(data).map(|parsed| {
                if new_cache.exists() {
                    if let Err(e) = std::fs::rename(&new_cache, &cache) {
                        warn!(
                            "Failed to rename from `{}` to `{}`, due to: {:?}",
                            new_cache.display(),
                            cache.display(),
                            e
                        )
                    }
                }

                parsed
            })
        }
        (Ok(data), None) => parse_data(data),
        (Err(e), Some(cache)) => {
            warn!("Failed to fetch labels for network {} due to: {:?}", network, e);
            warn!("Trying to load from cache `{}`", cache.display());

            let contents = std::fs::read_to_string(cache)?;
            parse_data(contents)
        }
        (Err(e), None) => return Err(e),
    }
}

async fn fetch_data(network: &String) -> anyhow::Result<String> {
    let file_path = format!("node-labels/{}.yaml", network);
    let mut content = octocrab::instance()
        .repos("dfinity", "dre")
        .get_content()
        .path(&file_path)
        .r#ref("main")
        .send()
        .await?;

    let items = content.take_items();

    match items.first() {
        Some(f) => match f.decoded_content() {
            Some(c) => Ok(c),
            None => Err(anyhow::anyhow!("Couldn't decode file: {}", file_path)),
        },
        None => Err(anyhow::anyhow!("File not found on path: {}", file_path)),
    }
}

fn parse_data(contents: String) -> anyhow::Result<Vec<Guest>> {
    let data = match serde_yaml::from_str::<serde_yaml::Value>(&contents)? {
        serde_yaml::Value::Mapping(c) => c,
        _ => return Err(anyhow::anyhow!("Failed to parse node labels file")),
    };

    let data = match data.get("data") {
        Some(serde_yaml::Value::Mapping(c)) => match c.get("v1") {
            Some(serde_yaml::Value::Mapping(c)) => c,
            _ => return Err(anyhow::anyhow!("Couldn't find data.v1 in node labels file")),
        },
        _ => return Err(anyhow::anyhow!("Couldn't find data in node labels file")),
    };

    Ok(data
        .iter()
        .map(|(key, value)| {
            let ip = key.as_str().unwrap();
            let dc = value.get("dc").unwrap().as_str().unwrap();
            let label = value.get("label").unwrap().as_str().unwrap();

            Guest {
                datacenter: dc.to_string(),
                ipv6: ip.parse().unwrap(),
                name: format!("{}-{}", dc, label),
                dfinity_owned: DFINITY_DCS.contains(dc),
            }
        })
        .collect())
}
