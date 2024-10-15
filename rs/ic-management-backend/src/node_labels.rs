use std::path::PathBuf;

use ic_management_types::Guest;
use log::info;

use crate::registry::DFINITY_DCS;

pub async fn query_guests(network: &String, local_cache: Option<PathBuf>, offline: bool) -> anyhow::Result<Vec<Guest>> {
    if offline {
        match local_cache {
            None => return Err(anyhow::anyhow!("No local cache file provided for offline mode.")),
            Some(path) => {
                info!("Loading labels from cache `{}`", path.display());

                let contents = std::fs::read_to_string(path)?;
                parse_data(contents)
            }
        }
    } else {
        let data = fetch_data(network).await?;
        if let Some(path) = local_cache {
            std::fs::write(path, &data)?;
        }
        parse_data(data)
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

pub fn parse_data(contents: String) -> anyhow::Result<Vec<Guest>> {
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
