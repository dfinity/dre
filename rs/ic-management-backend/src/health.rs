use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};

use ic_base_types::PrincipalId;
use ic_management_types::{Network, Status};
use prometheus_http_query::{Client, Selector};

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
        let ic_name = self.network.legacy_name();
        let subnet_name = subnet.to_string();
        let query_up = Selector::new()
            .metric("up")
            .eq("ic", ic_name.as_str())
            .eq("job", "replica")
            .eq("ic_subnet", subnet_name.as_str());

        let response_up = self.client.query(query_up).get().await?;
        let instant_up = response_up.data().as_vector().expect("Expected instant vector");

        // Alerts are synthetic time series and cannot be queries as regular metrics
        // https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/#inspecting-alerts-during-runtime
        let query_alert = format!(
            "ALERTS{{ic=\"{}\", job=\"replica\", ic_subnet=\"{}\", alertstate=\"firing\"}}",
            self.network.legacy_name(),
            subnet
        );
        let response_alert = self.client.query(query_alert).get().await?;
        let instant_alert = response_alert.data().as_vector().expect("Expected instant vector");
        let node_ids_with_alerts: HashSet<PrincipalId> = instant_alert
            .iter()
            .filter_map(|r| r.metric().get("ic_node").and_then(|id| PrincipalId::from_str(id).ok()))
            .collect();

        Ok(instant_up
            .iter()
            .filter_map(|r| {
                r.metric()
                    .get("ic_node")
                    .and_then(|id| PrincipalId::from_str(id).ok())
                    .map(|id| {
                        let status = if r.sample().value() == 1.0 {
                            if node_ids_with_alerts.contains(&id) {
                                Status::Degraded
                            } else {
                                Status::Healthy
                            }
                        } else {
                            Status::Dead
                        };
                        (id, status)
                    })
            })
            .collect())
    }

    pub async fn nodes(&self) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        let query = format!(
            r#"ic_replica_orchestrator:health_state:bottomk_1{{ic="{network}"}}"#,
            network = self.network.legacy_name(),
        );
        let response = self.client.query(query).get().await?;
        let results = response.data().as_vector().expect("Expected instant vector");
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
