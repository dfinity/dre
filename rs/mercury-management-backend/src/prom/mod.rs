use crate::backend_type;
use actix_web::error;
use anyhow::anyhow;
use async_trait::async_trait;
use backend_type::QueryType;
use chrono::{DateTime, Duration, Utc};
use derive_more::{Display, Error};
use futures::future::try_join_all;
#[allow(unused_imports)]
use futures_util::StreamExt;
#[allow(unused_imports)]
use hyper::client::HttpConnector;
use ic_base_types::PrincipalId;
use ic_nns_governance::pb::v1::ProposalInfo;
use itertools::{izip, Itertools};
use mercury_management_types::{Health, Subnet};
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
use reqwest::Client;
use serde_json::Value;
use std::clone::Clone;
use std::cmp;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::vec::Vec;
use url::ParseError;
use url::Url;
use urlencoding::encode;

#[derive(Debug, Display, Error)]
#[display(fmt = "my error: {}", name)]
pub struct TextError {
    pub name: &'static str,
}

pub struct HealthResponses {
    avg_finalization: f64,
    node_finalization: Vec<serde_json::Value>,
    node_connectedness: Vec<serde_json::Value>,
}

impl error::ResponseError for TextError {}

// Allowing unused here in case we want to refactor the way we currently do
// calls and to make Clippy happy.
#[allow(unused)]
pub struct PromCallBuilder<'a> {
    query_string: Option<String>,
    query_string_parameters: Option<Vec<String>>,
    query_params: Option<Vec<(String, String)>>,
    query_type: backend_type::QueryType,
    range_opts: Option<(DateTime<Utc>, DateTime<Utc>, i32)>,
    client: &'a PromClient,
}

// See above comment
#[allow(unused)]
impl<'a> PromCallBuilder<'a> {
    fn new(client: &'a PromClient) -> PromCallBuilder<'a> {
        PromCallBuilder {
            query_string: None,
            query_string_parameters: None,
            query_params: None,
            query_type: QueryType::Instant,
            range_opts: None,
            client,
        }
    }
    fn range(&mut self) -> &mut Self {
        self.query_type = QueryType::Range;
        self
    }

    fn instant(&mut self) -> &mut Self {
        self.query_type = QueryType::Range;
        self
    }

    fn query(&mut self, query_string: String, query_string_parameters: Vec<String>) -> &mut Self {
        self.query_string = Some(query_string);
        self.query_string_parameters = Some(query_string_parameters);
        self
    }

    fn params(&mut self, params: Vec<(String, String)>) -> &mut Self {
        self.query_params = Some(params);
        self
    }

    fn range_params(&mut self, range: (DateTime<Utc>, DateTime<Utc>, i32)) -> &mut Self {
        self.range_opts = Some(range);
        self
    }

    async fn call(&self) -> Result<Value, anyhow::Error> {
        let query_string = self.query_string.as_ref().expect("Query String required");
        let query_string_parameters = self
            .query_string_parameters
            .as_ref()
            .expect("Query string replacements needed");
        let query_params: Vec<(String, String)> = self.query_params.clone().unwrap_or_default();
        let _consumer = query_string_parameters
            .iter()
            .map(|x| query_string.replacen("{}", x, 1));
        match self.query_type.clone() {
            QueryType::Range => {
                return self
                    .client
                    .make_range_query_call(query_string.to_string(), query_params, self.range_opts)
                    .await;
            }
            QueryType::Instant => {
                return self
                    .client
                    .make_instant_query_call(query_string.to_string(), query_params)
                    .await;
            }
        }
    }
}

pub struct PromClient {
    client: Client,
    host: Url,
    //reserved for future use re: reqwests
    #[allow(unused)]
    query_timeout: Option<Duration>,
}
#[async_trait]
pub trait ICProm {
    async fn matching_single_query_call(
        &self,
        params: backend_type::ICNetworkQuerySingle,
    ) -> Result<Value, anyhow::Error>;
    async fn matching_aggregate_query_call(
        &self,
        params: backend_type::ICNetworkQueryAggregate,
    ) -> Result<Vec<Value>, anyhow::Error>;
}
impl PromClient {
    pub fn new(host: &str, query_timeout: Option<Duration>) -> Result<PromClient, ParseError> {
        let client = Client::new();
        let hosturl = Url::parse(host);
        match hosturl {
            Err(e) => Err(e),
            Ok(v) => Ok({
                PromClient {
                    client,
                    host: v,
                    query_timeout,
                }
            }),
        }
    }

    pub async fn make_range_query_call(
        &self,
        query: String,
        query_params: Vec<(String, String)>,
        range: Option<(DateTime<Utc>, DateTime<Utc>, i32)>,
    ) -> Result<Value, anyhow::Error> {
        let api_url = self.host.join("/api/v1/query_range")?;
        let defaulted_range: (DateTime<Utc>, DateTime<Utc>, i32) = range
            .or_else(|| {
                let now: DateTime<Utc> = Utc::now();
                let last_30 = now.checked_sub_signed(Duration::seconds(30))?;
                let step = 1;
                Some((now, last_30, step))
            })
            .unwrap();
        let resp = self
            .client
            .get(api_url)
            .query(&query_params)
            .query(&[("query", encode(&query))])
            .query(&[
                ("start", encode(&defaulted_range.0.to_rfc3339())),
                ("stop", encode(&defaulted_range.1.to_rfc3339())),
                ("step", encode(&defaulted_range.2.to_string())),
            ])
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(resp)
    }
    //Allowing here for future refactoring
    #[allow(unused)]
    pub fn req(&self) -> PromCallBuilder {
        PromCallBuilder::new(self)
    }

    pub async fn make_instant_query_call(
        &self,
        query: String,
        query_params: Vec<(String, String)>,
    ) -> Result<Value, anyhow::Error> {
        let api_url = self.host.join("/api/v1/query")?;
        let resp = self
            .client
            .get(api_url)
            .query(&query_params)
            .query(&[("query", encode(&query))])
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(resp)
    }
}

#[async_trait]
impl ICProm for PromClient {
    async fn matching_single_query_call(
        &self,
        params: backend_type::ICNetworkQuerySingle,
    ) -> Result<Value, anyhow::Error> {
        let mut replacements: Vec<String> = vec![params.ic_network];
        if let Some(v) = params.interval {
            replacements.push(v)
        };
        let mut query_params = params.qparams.unwrap_or_default();
        let mut query_string = params.query.query_single().unwrap();
        let _replaced_qstring = replacements
            .iter()
            .map(|x| query_string = query_string.replacen("{}", x, 1));
        query_params.push(("query".to_string(), query_string));
        match params.qtype {
            backend_type::QueryType::Range => {
                let api_url = self.host.join("/api/v1/query_range")?;
                let defaulted_range: backend_type::PromQueryRanges = params
                    .range
                    .or_else(|| {
                        let now: DateTime<Utc> = Utc::now();
                        let last_30 = now.checked_sub_signed(Duration::seconds(30))?;
                        let step = 1;
                        Some(backend_type::PromQueryRanges {
                            start: now,
                            stop: last_30,
                            step,
                        })
                    })
                    .unwrap();
                let resp = self
                    .client
                    .get(api_url)
                    .query(&query_params)
                    .query(&[
                        ("start", encode(&defaulted_range.start.to_rfc3339())),
                        ("stop", encode(&defaulted_range.stop.to_rfc3339())),
                        ("step", encode(&defaulted_range.step.to_string())),
                    ])
                    .send()
                    .await?
                    .json::<Value>()
                    .await?;
                return Ok(resp);
            }
            backend_type::QueryType::Instant => {
                let api_url = self.host.join("/api/v1/query")?;
                let resp = self
                    .client
                    .get(api_url)
                    .query(&query_params)
                    .send()
                    .await?
                    .json::<Value>()
                    .await?;
                Ok(resp)
            }
        }
    }

    async fn matching_aggregate_query_call(
        &self,
        params: backend_type::ICNetworkQueryAggregate,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let qstrings = params.query.query_vector();
        let num_queries = qstrings.len();
        if params.ic_network.len() != num_queries
            || params.interval.len() != num_queries
            || params.qparams.len() != num_queries
            || params.qparams.len() != num_queries
            || params.range.len() != num_queries
            || params.qtype.len() != num_queries
        {
            return Err(anyhow!("All parameters must be present for all queries"));
        } else {
            let mut resps = Vec::new();
            for (qtype, qstring, range, qparams, ic_network, interval) in izip!(
                params.qtype,
                qstrings,
                params.range,
                params.qparams.clone(),
                params.ic_network,
                params.interval
            ) {
                let mut replacements: Vec<String> = vec![ic_network];
                if let Some(v) = interval {
                    replacements.push(v)
                };
                let mut query_params = qparams.unwrap_or_default();
                let mut query_string = qstring;
                let _replaced_qstring = replacements
                    .iter()
                    .map(|x| query_string = query_string.replacen("{}", x, 1));
                query_params.push(("query".to_string(), query_string));
                match qtype {
                    backend_type::QueryType::Range => {
                        let api_url = self.host.join("/api/v1/query_range")?;
                        let defaulted_range: backend_type::PromQueryRanges = range
                            .or_else(|| {
                                let now: DateTime<Utc> = Utc::now();
                                let last_30 = now.checked_sub_signed(Duration::seconds(30))?;
                                let step = 1;
                                Some(backend_type::PromQueryRanges {
                                    start: now,
                                    stop: last_30,
                                    step,
                                })
                            })
                            .unwrap();
                        let resp = self
                            .client
                            .get(api_url)
                            .query(&query_params)
                            .query(&[
                                ("start", encode(&defaulted_range.start.to_rfc3339())),
                                ("stop", encode(&defaulted_range.stop.to_rfc3339())),
                                ("step", encode(&defaulted_range.step.to_string())),
                            ])
                            .send()
                            .await?
                            .json::<Value>()
                            .await?;
                        resps.push(resp)
                    }
                    backend_type::QueryType::Instant => {
                        let api_url = self.host.join("/api/v1/query")?;
                        let resp = self
                            .client
                            .get(api_url)
                            .query(&query_params)
                            .send()
                            .await?
                            .json::<Value>()
                            .await?;
                        resps.push(resp);
                    }
                }
            }
            return Ok(resps);
        }
    }
}

pub struct SubnetUpgrade {
    pub subnet_principal: PrincipalId,
    pub old_version: String,
    pub new_version: String,
    pub upgraded: bool,
}

pub async fn subnets_upgraded(
    subnets: HashMap<PrincipalId, mercury_management_types::Subnet>,
    proposals: Vec<(ProposalInfo, UpdateSubnetReplicaVersionPayload)>,
) -> anyhow::Result<HashMap<PrincipalId, SubnetUpgrade>> {
    let client = reqwest::Client::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let query = subnets
        .values()
        .map(|subnet| {
            proposals
                .iter()
                .find(|(_, payload)| {
                    payload.subnet_id == subnet.principal
                        && payload.replica_version_id == subnet.replica_version
                })
                .map(|(proposal, _)| proposal.executed_timestamp_seconds)
                .map(|update_time| {
                    let after_upgrade_window = Duration::hours(12).num_seconds() as u64;
                    let before_upgrade_window = Duration::minutes(10).num_seconds() as u64;
                    format!(
                        r#"
                            sum without (ic_active_version) (
                                label_replace(
                                    (
                                        (
                                            (sum by (ic_subnet, ic_active_version) (max_over_time(ic_replica_info{{ ic = "mercury", ic_subnet = "{}", ic_active_version = "{}" }}[{}s] offset {}s)))
                                                or
                                            label_replace(label_replace(vector(0), "ic_subnet", "{}", "",""), "ic_active_version", "{}", "", "")
                                        )
                                        / ignoring(ic_active_version) group_left
                                        sum by (ic_subnet, ic_active_version) (max_over_time(ic_replica_info{{ ic = "mercury", ic_subnet = "{}", ic_active_version != "{}" }}[{}s] offset {}s))
                                    )
                                    , "ic_old_version", "$1", "ic_active_version", "(.+)"
                                )
                            )
                        "#,
                        subnet.principal,
                        subnet.replica_version,
                        after_upgrade_window,
                        now.saturating_sub(update_time).saturating_sub(after_upgrade_window) + 1, // offset has to be at least 1
                        subnet.principal,
                        subnet.replica_version,
                        subnet.principal,
                        subnet.replica_version,
                        before_upgrade_window,
                        now.saturating_sub(update_time) + 1, // offset has to be at least 1
                    )
                }).unwrap_or(format!(r#"clamp_max(absent(dummy{{ic_old_version="-", ic_subnet="{subnet}"}}),1)"#, subnet=subnet.principal))
        })
        .collect::<Vec<_>>()
        .join(" or ");

    let request = client
        .get("http://prometheus.dfinity.systems:9090/api/v1/query")
        .query(&[("query", query)])
        .build()
        .expect("malformed request");

    let response = client
        .execute(request)
        .await?
        .json::<Value>()
        .await
        .expect("expected JSON response");

    let statuses = response["data"]["result"]
        .as_array()
        .expect("result present")
        .iter()
        .map(|m| {
            let subnet_principal = PrincipalId::from_str(
                m.clone()["metric"]["ic_subnet"]
                    .as_str()
                    .expect("expected principal string"),
            )
            .expect("unable to parse node principal");

            let old_version = m.clone()["metric"]["ic_old_version"]
                .as_str()
                .expect("expected instance string")
                .to_string();
            let upgraded = m.clone()["value"].as_array().expect("expected an array of values")[1]
                .as_str()
                .expect("expected stringified float")
                .parse::<f64>()
                .expect("expected float")
                >= 1.0;

            let new_version = subnets
                .get(&subnet_principal)
                .expect("subnet lost")
                .replica_version
                .clone();
            (
                subnet_principal,
                SubnetUpgrade {
                    subnet_principal,
                    old_version,
                    new_version,
                    upgraded,
                },
            )
        })
        .collect();
    Ok(statuses)
}

pub async fn node_healths_per_subnet(subnet: Subnet) -> anyhow::Result<Vec<(PrincipalId, Health)>> {
    // These really should be serde structs, but we're going to get it working
    // first.
    const EPSILON_FINALIZATION: f64 = 0.90;
    const DEAD_FINALIZATION: f64 = 0.40;
    const EPSILON_CONNECTEDNESS: f64 = 0.9;
    const DEAD_CONNECTEDNESS: f64 = 0.7;
    let subnet_id = subnet.principal.to_string();
    let required_nodes = subnet
        .nodes
        .iter()
        .map(|node| node.principal)
        .collect::<Vec<PrincipalId>>();
    let responses = node_health_indicator_queries(&subnet_id).await?;
    let finalization_subnet_average = responses.avg_finalization;
    let finalization_health_vec: Vec<Value> = responses.node_finalization;
    let connectedness_health_vec: Vec<Value> = responses.node_connectedness;
    let mut connectedness_vec: Vec<(PrincipalId, Option<Health>)> = Vec::new();
    let mut finalizations: Vec<(PrincipalId, Option<Health>)> = Vec::new();
    let expected_connectedness = connectedness_health_vec.len() as f64;
    for connectedness_data in connectedness_health_vec {
        let node: PrincipalId = PrincipalId::from_str(
            connectedness_data["metric"]["ic_node"]
                .as_str()
                .expect("IC Node is not a string - Bad response from Prometheus Server"),
        )
        .expect(
            "Can't parse returned node string into Principal from Prometheus - Bad response from Prometheus Server",
        );
        let connectedness: f64 = connectedness_data["value"][1]
            .as_str()
            .expect("Connectedness data not string-encoded. - Bad response from Prometheus Server")
            .parse::<f64>()
            .expect("Connectedness Data String not parseable as an f64 - Bad response from Prometheus Server");
        let health: Option<Health> = if (connectedness / expected_connectedness) < DEAD_CONNECTEDNESS {
            Some(Health::Offline)
        } else if (connectedness / expected_connectedness) < EPSILON_CONNECTEDNESS {
            Some(Health::Degraded)
        } else {
            Some(Health::Healthy)
        };
        connectedness_vec.push((node, health.clone()))
    }

    for finalization_data in finalization_health_vec {
        let node: PrincipalId = PrincipalId::from_str(
            finalization_data["metric"]["ic_node"]
                .as_str()
                .expect("IC_NODE label not str - Bad response from Prometheus Server"),
        )
        .expect("Invalid Node Principal - Bad response from Prometheus Server");
        let node_finalization: f64 = finalization_data["value"][1]
            .as_str()
            .expect("node_finalization not string-encoded - Bad response from Prometheus Server")
            .parse::<f64>()
            .expect("Cannot parse node finalization as f64 - Bad response from Prometheus Server");
        let finalization_health: Option<Health> =
            if (node_finalization / finalization_subnet_average) < DEAD_FINALIZATION {
                Some(Health::Offline)
            } else if (node_finalization / finalization_subnet_average) < EPSILON_FINALIZATION {
                Some(Health::Degraded)
            } else {
                Some(Health::Healthy)
            };
        finalizations.push((node, finalization_health));
    }
    let mut health_map: HashMap<PrincipalId, Option<Health>> = finalizations
        .iter()
        .map(|(node, health)| (node.to_owned(), health.to_owned()))
        .collect();
    for (node, data) in connectedness_vec {
        let curr_node_finalization_health = health_map.get(&node).unwrap();
        let extracted_curr_node_finalization_health = match curr_node_finalization_health {
            None => Health::Offline,
            Some(status) => status.clone(),
        };
        let extracted_curr_node_connectness_health = match data {
            None => Health::Offline,
            Some(status) => status,
        };
        health_map.insert(
            node,
            Some(cmp::min(
                extracted_curr_node_finalization_health.clone(),
                extracted_curr_node_connectness_health.clone(),
            )),
        );
    }
    for principal in required_nodes {
        health_map.entry(principal).or_insert(Some(Health::Offline));
    }
    Ok(health_map
        .iter()
        .map(|(node, health)| (node.to_owned(), health.as_ref().unwrap().clone()))
        .collect())
}

pub async fn node_health_indicator_queries(subnet_id: &str) -> Result<HealthResponses, anyhow::Error> {
    const NODE_STATUS_CONNECTED_EXPORTER: i32 = 3;
    let client = reqwest::Client::new();
    let avg_request = client
        .get("http://prometheus.dfinity.systems:9090/api/v1/query")
        .query(&[("query",
            &format!(r#"avg(rate(artifact_pool_consensus_height_stat{{ic="mercury", ic_subnet="{}", type="finalization", stat="max", pool_type="validated"}}[5m]))"#,
            &subnet_id))])
        .build()
        .expect("Error building request");
    let avg_execute = client.execute(avg_request);

    let finalization_per_node_request = client
        .get("http://prometheus.dfinity.systems:9090/api/v1/query")
        .query(&
            [("query",
            &format!(r#"rate(artifact_pool_consensus_height_stat{{ic="mercury", ic_subnet="{}", type="finalization", stat="max", pool_type="validated"}}[1h])"#,
            &subnet_id))])
        .build()
        .expect("Error building request");
    let finalization_per_node_execute = client.execute(finalization_per_node_request);

    let connectedness_per_node_request = client
        .get("http://prometheus.dfinity.systems:9090/api/v1/query")
        .query(&[(
            "query",
            &format!(
                r#"count by (ic_node) (transport_flow_state{{ic_subnet="{}"}}=={})"#,
                &subnet_id, NODE_STATUS_CONNECTED_EXPORTER
            ),
        )])
        .build()
        .expect("Error building request");
    let connectedness_per_node_execute = client.execute(connectedness_per_node_request);
    let (avg_finalization, node_finalization, node_connectedness) = match try_join_all(vec![
        avg_execute,
        finalization_per_node_execute,
        connectedness_per_node_execute,
    ])
    .await?
    .into_iter()
    .next_tuple()
    {
        Some((first, second, third)) => (first, second, third),
        _ => unreachable!(),
    };
    let avg_finalization: serde_json::Value = avg_finalization.json().await.expect("Response was not valid JSON");
    let node_finalization: serde_json::Value = node_finalization.json().await.expect("Response was not valid JSON");
    let node_connectedness: serde_json::Value = node_connectedness.json().await.expect("Response was not valid JSON");
    let avg_finalization = avg_finalization["data"]["result"][0]["value"][1].clone();
    let node_finalization: Vec<serde_json::Value> = node_finalization["data"]["result"].as_array().unwrap().to_vec();
    let node_connectedness = node_connectedness["data"]["result"].as_array().unwrap().to_vec();
    Ok(HealthResponses {
        avg_finalization: avg_finalization
            .as_str()
            .unwrap()
            .parse::<f64>()
            .expect("unable to parse as f64"),
        node_finalization: node_finalization.to_vec(),
        node_connectedness: node_connectedness.to_vec(),
    })
}
