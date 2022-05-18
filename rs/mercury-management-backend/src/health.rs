use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use ic_base_types::PrincipalId;
use prometheus_http_query::{Client, InstantVector, Selector};
use serde::Serialize;
use strum_macros::EnumString;

#[derive(EnumString, Serialize)]
pub enum Status {
    Unhealthy = 0,
    Healthy = 1,
    Unknown = 2,
}

fn prometheus_client() -> Client {
    Client::try_from("http://prometheus.dfinity.systems:9090").unwrap()
}

pub async fn subnet(subnet: PrincipalId) -> anyhow::Result<HashMap<PrincipalId, Status>> {
    let client = prometheus_client();

    let query: InstantVector = Selector::new()
        .metric("up")
        .with("ic", "mercury")
        .with("job", "replica")
        .with("ic_subnet", subnet.to_string().as_str())
        .try_into()?;

    let response = client.query(query, None, None).await?;
    let results = response.as_instant().expect("Expected instant vector");
    Ok(results
        .iter()
        .filter_map(|r| {
            let status = if r.sample().value() == 1.0 {
                Status::Healthy
            } else {
                Status::Unhealthy
            };
            r.metric()
                .get("ic_node")
                .and_then(|id| PrincipalId::from_str(id).ok())
                .map(|id| (id, status))
        })
        .collect())
}

pub async fn nodes() -> anyhow::Result<HashMap<PrincipalId, Status>> {
    let client = Client::try_from("http://prometheus.dfinity.systems:9090").unwrap();

    let query: InstantVector = InstantVector(
        r#"
            bottomk by(ic_node) (1,
                (
                    clamp_max(sum by (ic_node) (ALERTS{alertstate="firing", ic="mercury", ic_node=~".+"}), 0)
                )
                    or
                (
                    sum by(ic_node) (
                        up{job="orchestrator", ic="mercury"}
                    )
                )
            )
        "#
        .to_string(),
    );
    let response = client.query(query, None, None).await?;
    let results = response.as_instant().expect("Expected instant vector");
    Ok(results
        .iter()
        .filter_map(|r| {
            let status = if r.sample().value() == 1.0 {
                Status::Healthy
            } else {
                Status::Unhealthy
            };
            r.metric()
                .get("ic_node")
                .map(|id| (PrincipalId::from_str(id).unwrap(), status))
        })
        .collect())
}
