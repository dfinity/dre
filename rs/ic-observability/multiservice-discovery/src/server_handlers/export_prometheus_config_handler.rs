use super::Server;
use crate::definition::{api_boundary_nodes_target_dtos_from_definitions, RunningDefinition};
use crate::{
    definition::{boundary_nodes_from_definitions, ic_node_target_dtos_from_definitions},
    TargetFilterSpec,
};
use axum::{
    extract::{Query, State},
    http::header,
    http::{HeaderMap, StatusCode},
};
use multiservice_discovery_shared::builders::prometheus_config_structure::{map_target_group, PrometheusStaticConfig};
use std::collections::BTreeMap;

pub fn serialize_definitions_to_prometheus_config(definitions: BTreeMap<String, RunningDefinition>, filters: TargetFilterSpec) -> (usize, String) {
    let ic_node_targets: Vec<PrometheusStaticConfig> =
        map_target_group(ic_node_target_dtos_from_definitions(&definitions, &filters).into_iter().collect());

    let boundary_nodes_targets = boundary_nodes_from_definitions(&definitions, &filters)
        .iter()
        .map(|(definition_name, bn)| PrometheusStaticConfig {
            targets: bn.targets.clone().iter().map(|g| bn.job_type.url(*g, true)).collect(),
            labels: {
                BTreeMap::from([
                    ("ic", definition_name.clone()),
                    ("name", bn.name.clone()),
                    ("job", bn.job_type.to_string()),
                ])
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .chain(bn.custom_labels.clone())
                .collect::<BTreeMap<_, _>>()
            },
        })
        .collect();

    let api_boundary_nodes_targets: Vec<PrometheusStaticConfig> = map_target_group(
        api_boundary_nodes_target_dtos_from_definitions(&definitions, &filters)
            .into_iter()
            .collect(),
    );

    let total_targets = [ic_node_targets, boundary_nodes_targets, api_boundary_nodes_targets].concat();

    (total_targets.len(), serde_json::to_string_pretty(&total_targets).unwrap())
}

pub(super) async fn export_prometheus_config(
    State(binding): State<Server>,
    filters: Query<TargetFilterSpec>,
) -> Result<(HeaderMap, String), (StatusCode, String)> {
    let definitions = binding.supervisor.definitions.lock().await;
    let (targets_len, text) = serialize_definitions_to_prometheus_config(definitions.clone(), filters.0);
    if targets_len > 0 {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        Ok((headers, text))
    } else {
        Err((StatusCode::NOT_FOUND, "No targets found".to_string()))
    }
}
