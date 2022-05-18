use anyhow::Error;
use chrono::serde::ts_seconds;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct ICNetworkQuerySingle {
    pub ic_network: String,
    pub interval: Option<String>,
    pub query: ICPromInstantQueries,
    pub qparams: Option<Vec<(String, String)>>,
    pub range: Option<PromQueryRanges>,
    pub qtype: QueryType,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ICNetworkQueryAggregate {
    pub ic_network: Vec<String>,
    pub interval: Vec<Option<String>>,
    pub query: ICPromAggregateQueries,
    pub qparams: Vec<Option<Vec<(String, String)>>>,
    pub range: Vec<Option<PromQueryRanges>>,
    pub qtype: Vec<QueryType>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub enum QueryType {
    Range,
    Instant,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct PromQueryParams {
    pub query: String,
    pub qtype: QueryType,
    pub replacements: Option<Vec<String>>,
    pub qparams: Option<Vec<(String, String)>>,
    pub range: Option<PromQueryRanges>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PromQueryRanges {
    #[serde(with = "ts_seconds")]
    pub start: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    pub stop: DateTime<Utc>,
    pub step: i32,
}

#[derive(Deserialize, Clone, Debug, Serialize)]
pub enum ICPromInstantQueries {
    GetTotalFinalizationBlockrate,
    GetFinalizationBlockrateBySubnet,
    GetMessageExecRate,
    GetSubnetUptimeByNode,
    GetNeededReplicas,
    GetCurrentReplicas,
    GetHttpRequests,
    GetGossipBytes,
    GetGossipArtifacts,
    ReplicaResidentMemory,
    CanisterMemoryUsageBySubnet,
    ReplicaThreads,
    MeanNodeCpuMode,
    NodeKernelVersions,
}

impl ICPromInstantQueries {
    pub fn query_single(self) -> Result<String, Error> {
        match self {
            ICPromInstantQueries::GetTotalFinalizationBlockrate => { Ok(r#"avg(avg by (ic_subnet) (artifact_pool_consensus_height_stat{{job="replica",type="finalization",pool_type="validated",stat="max", ic="{}"}}))"#.to_string()) },
            ICPromInstantQueries::GetFinalizationBlockrateBySubnet => { Ok(r#"avg by (ic_subnet) (artifact_pool_consensus_height_stat{{job="replica",type="finalization",pool_type="validated",stat="max", ic="{}"}})"#.to_string())},
            ICPromInstantQueries::GetMessageExecRate => { Ok(r#"quantile by (ic_subnet) (0.5, rate(scheduler_message_execution_duration_seconds_count{{job="replica",ic="{}"}}[30s]))"#.to_string()) },
            ICPromInstantQueries::GetSubnetUptimeByNode => { Ok(r#"up{{job="replica", ic="{}"}}"#.to_string()) },
            ICPromInstantQueries::GetNeededReplicas => { Ok(r#"count(up{{job="replica",ic="{}"}})"#.to_string()) },
            ICPromInstantQueries::GetCurrentReplicas => {  Ok(r#"sum by(ic_subnet) (up{{job="replica",ic="{}"}})"#.to_string())},
            ICPromInstantQueries::GetHttpRequests => { Ok(r#"sum by (request_type, type, status) (rate(replica_http_request_duration_seconds_count{{ic="{}"}}))"#.to_string()) },
            ICPromInstantQueries::GetGossipBytes => { Ok(r#"quantile by (pool, pool_type) (0.5,  rate(artifact_pool_received_artifact_bytes_sum{{ic="{}", pool_type="unvalidated"}}[$period]))"#.to_string()) },
            ICPromInstantQueries::GetGossipArtifacts => { Ok(r#"quantile by (pool, pool_type) (0.5,  rate(artifact_pool_received_artifact_bytes_count{{ic="{}", pool_type="unvalidated"}}[$period]))"#.to_string()) },
            ICPromInstantQueries::ReplicaResidentMemory => { Ok(r#"process_resident_memory_bytes{ic="{}", job="replica"}"#.to_string()) },
            ICPromInstantQueries::CanisterMemoryUsageBySubnet => { Ok(r#"quantile by (ic_subnet) (0.5, canister_memory_usage_bytes{job="replica",ic="{}"})"#.to_string()) },
            ICPromInstantQueries::ReplicaThreads => { Ok(r#"process_threads{job="replica",ic="{}"}"#.to_string()) },
            ICPromInstantQueries::MeanNodeCpuMode => { Ok(r#"sum by(mode) (rate(node_cpu_seconds_total{job=~"node_exporter.*",ic="{}"}[{}]))/scalar(count(up{job=~"node_exporter.*",ic="{}"}))")"#.to_string()) },
            ICPromInstantQueries::NodeKernelVersions => { Ok(r#"count by(version) (  rate(node_uname_info{job=~"node_exporter.*",ic="{}"}[{}}]))"#.to_string())        }
    }
    }
}

#[derive(Deserialize, Clone, Debug, Serialize)]
pub enum ICPromAggregateQueries {
    GetTransportFlowsByState,
    MedianResidentMemory,
    MeanNodeNetworkTraffic,
}

impl ICPromAggregateQueries {
    pub fn query_vector(self) -> Vec<String> {
        match self {
            ICPromAggregateQueries::GetTransportFlowsByState => {
                vec![
                    r#"count(transport_flow_state{{job="replica",ic="{}"}} == 3) or on() vector(0)"#.to_string(),
                    r#"count(transport_flow_state{{job="replica",ic="{}"}} == 1) or on() vector(0)"#.to_string(),
                    r#"count(transport_flow_state{{job="replica",ic="{}"}} == 2) or on() vector(0)"#.to_string(),
                    r#"count by(instance) (transport_flow_state{{job="replica",ic="{}"}} == 3)"#.to_string(),
                ]
            }
            ICPromAggregateQueries::MedianResidentMemory => {
                vec![r#"quantile(0.5, canister_memory_usage_bytes{job="replica",ic="{}"})"#.to_string(),
                                           r#"quantile(0.5, current_heap_delta{job="replica",ic="{}"})"#.to_string(),
                                           r#"quantile(0.5, sum(artifact_pool_artifact_bytes{job="replica",ic="{}"}))"#.to_string(),
                                           r#"quantile(0.5, process_resident_memory_bytes{job="replica",ic="{}"}) - quantile(0.5, current_heap_delta{job="replica",ic="{}"}) - quantile(0.5, canister_memory_usage_bytes{job="replica",ic="{}"})""#.to_string()]
            }
            ICPromAggregateQueries::MeanNodeNetworkTraffic => {
                vec![
                    r#"avg(rate(node_network_transmit_packets_total{job=~"node_exporter.*",ic="{}"}[{}]))"#.to_string(),
                    r#"avg(rate(node_network_transmit_bytes_total{job=~"node_exporter.*",ic="{}"}[{}]))"#.to_string(),
                    r#"avg(rate(node_network_receive_packets_total{job=~"node_exporter.*",ic="{}"}[{}]))"#.to_string(),
                    r#"avg(rate(node_network_receive_bytes_total{job=~"node_exporter.*",ic="{}"}[{}]))"#.to_string(),
                ]
            }
        }
    }
}
