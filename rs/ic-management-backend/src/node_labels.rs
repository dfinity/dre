use ic_management_types::Guest;

use crate::registry::DFINITY_DCS;

pub async fn query_guests(network: &String) -> anyhow::Result<Vec<Guest>> {
    let file_path = format!("node-labels/{}.yaml", network);
    let mut content = octocrab::instance()
        .repos("dfinity", "dre")
        .get_content()
        .path(&file_path)
        .r#ref("main")
        .send()
        .await?;

    let items = content.take_items();

    let data = match items.first() {
        Some(f) => match f.decoded_content() {
            Some(c) => match serde_yaml::from_str::<serde_yaml::Value>(&c)? {
                serde_yaml::Value::Mapping(c) => c,
                _ => return Err(anyhow::anyhow!("Failed to parse file: {}", file_path)),
            },
            None => return Err(anyhow::anyhow!("Couldn't decode file: {}", file_path)),
        },
        None => return Err(anyhow::anyhow!("File not found on path: {}", file_path)),
    };
    let data = match data.get("data") {
        Some(serde_yaml::Value::Mapping(c)) => match c.get("v1") {
            Some(serde_yaml::Value::Mapping(c)) => c,
            _ => return Err(anyhow::anyhow!("Couldn't find data.v1 in file: {}", file_path)),
        },
        _ => return Err(anyhow::anyhow!("Couldn't find data in file: {}", file_path)),
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
                name: label.to_string(),
                // Unknown fields
                dfinity_owned: DFINITY_DCS.contains(dc),
                decentralized: true,
            }
        })
        .collect())
}
