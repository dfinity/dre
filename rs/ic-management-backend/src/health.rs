use futures::future::BoxFuture;
use indexmap::IndexMap;
use mockall::automock;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, str::FromStr};

use ic_base_types::PrincipalId;
use ic_management_types::{HealthStatus, Network};
use log::warn;
use prometheus_http_query::{Client, Selector};
use reqwest::{Client as ReqwestClient, Method};
use serde_json::Value;
use url::Url;

use crate::prometheus;

pub struct HealthClient {
    implementation: HealthStatusQuerierImplementations,
    local_cache: Option<PathBuf>,
    offline: bool,
}

impl HealthClient {
    pub fn new(network: Network, local_cache: Option<PathBuf>, offline: bool) -> Self {
        Self {
            implementation: network.into(),
            local_cache,
            offline,
        }
    }
}

impl HealthStatusQuerier for HealthClient {
    fn nodes_short_info(&self) -> BoxFuture<'_, anyhow::Result<Vec<ShortNodeInfo>>> {
        Box::pin(async {
            match (self.offline, self.local_cache.clone()) {
                // Offline and no cache is not possible
                (true, None) => Err(anyhow::anyhow!("Cannot run offline with no local cache destination")),
                // Should load from local cache
                (true, Some(path)) => {
                    let contents = fs_err::read_to_string(&path)?;
                    serde_json::from_str(&contents)
                        .map_err(|e| anyhow::anyhow!("Failed to deserialize from local cache on path `{}` due to: {:?}", path.display(), e))
                }
                // Runing online
                (false, local_cache) => {
                    let nodes = match &self.implementation {
                        HealthStatusQuerierImplementations::Dashboard(public_dashboard_health_client) => {
                            public_dashboard_health_client.get_all_nodes().await?
                        }
                        HealthStatusQuerierImplementations::Prometheus(prometheus_health_client) => prometheus_health_client.get_all_nodes().await?,
                    };

                    if let Some(path) = local_cache {
                        let contents = serde_json::to_string_pretty(&nodes)?;
                        fs_err::write(&path, &contents)
                            .map_err(|e| anyhow::anyhow!("Failed to update local cache on path `{}` due to: {:?}", path.display(), e))?;
                    }
                    Ok(nodes)
                }
            }
        })
    }
}

pub enum HealthStatusQuerierImplementations {
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

#[automock]
#[allow(dead_code)]
pub trait HealthStatusQuerier: Send + Sync {
    fn nodes_short_info(&self) -> BoxFuture<'_, anyhow::Result<Vec<ShortNodeInfo>>>;
    fn subnet(&self, subnet: PrincipalId) -> BoxFuture<'_, anyhow::Result<IndexMap<PrincipalId, HealthStatus>>> {
        Box::pin(async move {
            Ok(self
                .nodes_short_info()
                .await?
                .into_iter()
                .filter(|node| matches!(node.subnet_id, Some(s) if PartialEq::eq(&s, &subnet)))
                .map(|node_info| (node_info.node_id, node_info.status))
                .collect())
        })
    }
    fn nodes(&self) -> BoxFuture<'_, anyhow::Result<IndexMap<PrincipalId, HealthStatus>>> {
        Box::pin(async {
            Ok(self
                .nodes_short_info()
                .await?
                .into_iter()
                .map(|node_info| (node_info.node_id, node_info.status))
                .collect())
        })
    }
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

        let response_text = response.text().await.map_err(|e| anyhow::anyhow!("Error reading response text: {}", e))?;
        let response: Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::anyhow!("Error parsing json: {}. Raw text from the response: {}", e, response_text))?;

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

            let status = match (node.get("alertname"), node.get("status")) {
                (alertname, Some(s)) => {
                    let s = s.to_string();
                    let s = get_unquoted(&s);
                    let alertname = alertname.map(|a| a.to_string()).unwrap_or_default();
                    let alertname = get_unquoted(&alertname);
                    HealthStatus::from_str_from_dashboard(alertname, s)
                }
                (_, None) => {
                    warn!("Didn't find node while checking node health which shouldn't happen!");
                    continue;
                }
            };

            let node_dc = match node.get("dc_id") {
                None => {
                    warn!("Didn't find datacenter while checking node health which shouldn't happen!");
                    continue;
                }
                Some(dc) => {
                    let dc = dc.to_string();
                    let dc = get_unquoted(&dc);
                    dc.to_string()
                }
            };

            let status = if node_dc == "mn2" { HealthStatus::Healthy } else { status };

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

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortNodeInfo {
    pub node_id: PrincipalId,
    pub subnet_id: Option<PrincipalId>,
    pub status: HealthStatus,
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

impl PrometheusHealthClient {
    fn get_all_nodes(&self) -> BoxFuture<'_, anyhow::Result<Vec<ShortNodeInfo>>> {
        Box::pin(async move {
            let ic_name = self.network.legacy_name();
            let query_up = Selector::new().metric("up").eq("ic", ic_name.as_str()).eq("job", "replica");

            let response_up = self.client.query(query_up).get().await?;
            let instant_up = response_up.data().as_vector().expect("Expected instant vector");

            // Alerts are synthetic time series and cannot be queries as regular metrics
            // https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/#inspecting-alerts-during-runtime
            let query_alert = format!(
                "ALERTS{{ic=\"{}\", job=\"replica\", alertstate=\"firing\", alertname!=\"IC_PrometheusTargetMissing\"}}",
                self.network.legacy_name(),
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
                        let subnet_id = r.metric().get("ic_subnet").and_then(|id| PrincipalId::from_str(id).ok());
                        let status = if r.sample().value() == 1.0 {
                            if node_ids_with_alerts.contains(&id) {
                                HealthStatus::Degraded
                            } else {
                                HealthStatus::Healthy
                            }
                        } else {
                            HealthStatus::Dead
                        };
                        ShortNodeInfo {
                            node_id: id,
                            subnet_id,
                            status,
                        }
                    })
                })
                .collect())
        })
    }
}
