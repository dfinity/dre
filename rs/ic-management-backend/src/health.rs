use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};

use ic_base_types::PrincipalId;
use ic_management_types::{Network, Status};
use log::warn;
use prometheus_http_query::{Client, Selector};
use reqwest::{Client as ReqwestClient, Method};
use serde_json::Value;
use url::Url;

use crate::prometheus;

pub struct HealthClient {
    implementation: HealthStatusQuerierImplementations,
}

impl HealthClient {
    pub fn new(network: Network) -> Self {
        Self {
            implementation: network.into(),
        }
    }
}

impl HealthStatusQuerier for HealthClient {
    async fn subnet(&self, subnet: PrincipalId) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        match &self.implementation {
            HealthStatusQuerierImplementations::Dashboard(c) => c.subnet(subnet).await,
            HealthStatusQuerierImplementations::Prometheus(c) => c.subnet(subnet).await,
        }
    }

    async fn nodes(&self) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        match &self.implementation {
            HealthStatusQuerierImplementations::Dashboard(c) => c.nodes().await,
            HealthStatusQuerierImplementations::Prometheus(c) => c.nodes().await,
        }
    }
}

enum HealthStatusQuerierImplementations {
    Dashboard(PublicDashboardHealthClient),
    Prometheus(PrometheusHealthClient),
}

impl From<Network> for HealthStatusQuerierImplementations {
    fn from(value: Network) -> Self {
        if value.is_mainnet() {
            HealthStatusQuerierImplementations::Dashboard(PublicDashboardHealthClient::new(None))
        } else {
            HealthStatusQuerierImplementations::Prometheus(PrometheusHealthClient::new(value))
        }
    }
}

pub trait HealthStatusQuerier {
    fn subnet(&self, subnet: PrincipalId) -> impl std::future::Future<Output = anyhow::Result<BTreeMap<PrincipalId, Status>>> + Send;
    fn nodes(&self) -> impl std::future::Future<Output = anyhow::Result<BTreeMap<PrincipalId, Status>>> + Send;
}

pub struct PublicDashboardHealthClient {
    client: ReqwestClient,
    base_url: Url,
}

impl PublicDashboardHealthClient {
    pub fn new(base_url: Option<Url>) -> Self {
        Self {
            client: ReqwestClient::new(),
            base_url: match base_url {
                Some(u) => u,
                None => Url::from_str("https://ic-api.internetcomputer.org/").expect("Should be a valid url"),
            },
        }
    }

    fn api_node_list(&self) -> anyhow::Result<Url> {
        self.base_url
            .join("/api/node-list")
            .map_err(|e| anyhow::anyhow!("Error joining url: {:?}", e))
    }

    async fn get_all_nodes(&self) -> anyhow::Result<Vec<ShortNodeInfo>> {
        let request = self
            .client
            .request(Method::GET, self.api_node_list()?)
            .header("accept", "application/json")
            .build()
            .map_err(|e| anyhow::anyhow!("Error building a request: {:?}", e))?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while fetching data from public dashboard: {:?}", e))?;

        let response = response
            .json::<Value>()
            .await
            .map_err(|e| anyhow::anyhow!("Error unmarshaling json: {:?}", e))?;

        let nodes = match response.get("nodes") {
            None => return Err(anyhow::anyhow!("Unexpected data contract. Missing 'nodes' key.")),
            Some(v) => v,
        };

        let mut response = vec![];

        let nodes = match nodes.as_array() {
            None => return Err(anyhow::anyhow!("Unexpected data contract. Couldn't parse response as array")),
            Some(n) => n,
        };

        for node in nodes {
            let node_id = match node.get("node_id") {
                None => {
                    warn!("Didn't find pricipal while checking node health which shouldn't happen!");
                    continue;
                }
                Some(p) => {
                    // Serde to_string() returns quoted strings which means we have to skip first and last char.
                    let p = p.to_string();
                    let p = get_unquoted(&p);
                    match PrincipalId::from_str(p) {
                        Ok(p) => p,
                        Err(e) => {
                            warn!("Couldn't parse principal from string {} which shouldn't happen! Error: {:?}", p, e);
                            continue;
                        }
                    }
                }
            };

            let status = match node.get("status") {
                None => {
                    warn!("Didn't find node while checking node health which shouldn't happen!");
                    continue;
                }
                Some(s) => {
                    let s = s.to_string();
                    let s = get_unquoted(&s);
                    Status::from_str_from_dashboard(s)
                }
            };

            let maybe_subnet = match node.get("subnet_id") {
                None => None,
                Some(pr) => {
                    let p = pr.to_string();
                    let p = get_unquoted(&p);
                    match PrincipalId::from_str(p) {
                        Ok(p) => Some(p),
                        // Serde returns quoted strings but if the value is null it doesn't quote it, meaning we get (after skipping) 'ul'
                        Err(_) if p == "ul" => None,
                        Err(e) => {
                            warn!("Couldn't parse principal from string '{}' which shouldn't happen! Error: {:?}", p, e);
                            None
                        }
                    }
                }
            };

            response.push(ShortNodeInfo {
                node_id,
                subnet_id: maybe_subnet,
                status,
            })
        }

        Ok(response)
    }
}

fn get_unquoted(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

struct ShortNodeInfo {
    node_id: PrincipalId,
    subnet_id: Option<PrincipalId>,
    status: Status,
}

impl HealthStatusQuerier for PublicDashboardHealthClient {
    async fn subnet(&self, subnet: PrincipalId) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        Ok(self
            .get_all_nodes()
            .await?
            .into_iter()
            .filter(|n| match n.subnet_id {
                None => false,
                Some(p) => p.eq(&subnet),
            })
            .map(|n| (n.node_id, n.status))
            .collect())
    }

    async fn nodes(&self) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
        Ok(self.get_all_nodes().await?.into_iter().map(|n| (n.node_id, n.status)).collect())
    }
}

pub struct PrometheusHealthClient {
    client: Client,
    network: Network,
}

impl PrometheusHealthClient {
    pub fn new(network: Network) -> Self {
        Self {
            client: prometheus::client(&network),
            network,
        }
    }
}

impl HealthStatusQuerier for PrometheusHealthClient {
    async fn subnet(&self, subnet: PrincipalId) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
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
                r.metric().get("ic_node").and_then(|id| PrincipalId::from_str(id).ok()).map(|id| {
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

    async fn nodes(&self) -> anyhow::Result<BTreeMap<PrincipalId, Status>> {
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
                r.metric().get("ic_node").map(|id| (PrincipalId::from_str(id).unwrap(), status))
            })
            .collect())
    }
}
