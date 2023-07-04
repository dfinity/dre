use std::{collections::BTreeMap, convert::TryInto, str::FromStr};

use ic_base_types::PrincipalId;
use ic_management_types::{Network, Status};
use prometheus_http_query::{Client, InstantVector, Selector};

use crate::prometheus;

pub struct HealthClient {
    client: Client,
    network: Network,
}

impl HealthClient {
    pub fn new(network: Network) -> Self {
        Self {
            client: prometheus::client(&network),
            network,
        }
    }

    pub async fn subnet(&self, subnet: PrincipalId) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        let query: InstantVector = Selector::new()
            .metric("up")
            .with("ic", &self.network.legacy_name())
            .with("job", "replica")
            .with("ic_subnet", subnet.to_string().as_str())
            .try_into()?;

        let response = self.client.query(query, None, None).await?;
        let results = response.as_instant().expect("Expected instant vector");
        Ok(results
            .iter()
            .filter_map(|r| {
                let status = if r.sample().value() == 1.0 {
                    Status::Healthy
                } else {
                    Status::Dead
                };
                r.metric()
                    .get("ic_node")
                    .and_then(|id| PrincipalId::from_str(id).ok())
                    .map(|id| (id, status))
            })
            .collect())
    }

    pub async fn nodes(&self) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        let query: InstantVector = InstantVector(format!(
            r#"
                label_replace(
                    ((
                        sum by(ic_node) (
                            up{{job="replica", ic="{network}"}}
                        )
                            or
                        sum by(ic_node) (
                            up{{job="orchestrator", ic="{network}"}}
                        )
                    # swap 0 and 1 to get the health{{state="dead"}}=1 when up == 0
                    ) - 1)^2 == 1
                ,
                    "state", "Dead", "", ""
                )
                    or ignoring(state)
                label_replace(
                    clamp_max(count by (ic_node) (ALERTS{{ic="{network}", job!="host_node_exporter"}}), 1)
                ,
                    "state", "Degraded", "", ""
                )
                    or ignoring(state)
                label_replace(
                    (
                        sum by(ic_node) (
                            up{{job="replica", ic="{network}"}}
                        )
                            or
                        sum by(ic_node) (
                            up{{job="orchestrator", ic="{network}"}}
                        )
                    ) == 1
                ,
                    "state", "Healthy", "", ""
                )
            "#,
            network = self.network.legacy_name(),
        ));
        let response = self.client.query(query, None, None).await?;
        let results = response.as_instant().expect("Expected instant vector");
        Ok(results
            .iter()
            .filter_map(|r| {
                let status = Status::from_str(r.metric().get("state").expect("all vectors should have a state label"))
                    .expect("all vectors should have a valid label");
                r.metric()
                    .get("ic_node")
                    .map(|id| (PrincipalId::from_str(id).unwrap(), status))
            })
            .collect())
    }
}
