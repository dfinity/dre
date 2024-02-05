use crate::definition::RunningDefinition;

use super::Server;
use axum::extract::State;
use axum::http::StatusCode;
use multiservice_discovery_shared::builders::prometheus_config_structure::{map_target_group, PrometheusStaticConfig};
use multiservice_discovery_shared::contracts::target::{map_to_target_dto, TargetDto};
use service_discovery::job_types::{JobType, NodeOS};
use std::collections::BTreeMap;

pub fn serialize_definitions_to_prometheus_config(definitions: BTreeMap<String, RunningDefinition>) -> (usize, String) {
    let mut ic_node_targets: Vec<TargetDto> = vec![];

    for (_, def) in definitions.iter() {
        for job_type in JobType::all_for_ic_nodes() {
            let targets = match def.get_target_groups(job_type) {
                Ok(targets) => targets,
                Err(_) => continue,
            };

            targets.iter().for_each(|target_group| {
                if let Some(target) = ic_node_targets.iter_mut().find(|t| t.node_id == target_group.node_id) {
                    target.jobs.push(job_type);
                } else {
                    ic_node_targets.push(map_to_target_dto(
                        target_group,
                        job_type,
                        BTreeMap::new(),
                        target_group.node_id.to_string(),
                        def.name(),
                    ));
                }
            });
        }
    }

    let ic_node_targets: Vec<PrometheusStaticConfig> = map_target_group(ic_node_targets.into_iter().collect());

    let boundary_nodes_targets = definitions
        .iter()
        .flat_map(|(_, def)| {
            def.definition.boundary_nodes.iter().filter_map(|bn| {
                // Since boundary nodes have been checked for correct job
                // type when they were added via POST, then we can trust
                // the correct job type is at play here.
                // If, however, this boundary node is under the test environment,
                // and the job is Node Exporter, then skip adding this
                // target altogether.
                if bn
                    .custom_labels
                    .iter()
                    .any(|(k, v)| k.as_str() == "env" && v.as_str() == "test")
                    && bn.job_type == JobType::NodeExporter(NodeOS::Host)
                {
                    return None;
                }
                Some(PrometheusStaticConfig {
                    targets: bn.targets.clone().iter().map(|g| bn.job_type.url(*g, true)).collect(),
                    labels: {
                        BTreeMap::from([
                            ("ic", def.name()),
                            ("name", bn.name.clone()),
                            ("job", bn.job_type.to_string()),
                        ])
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .chain(bn.custom_labels.clone())
                        .collect::<BTreeMap<_, _>>()
                    },
                })
            })
        })
        .collect();

    let total_targets = [ic_node_targets, boundary_nodes_targets].concat();

    (
        total_targets.len(),
        serde_json::to_string_pretty(&total_targets).unwrap(),
    )
}

pub(super) async fn export_prometheus_config(State(binding): State<Server>) -> Result<String, (StatusCode, String)> {
    let definitions = binding.supervisor.definitions.lock().await;
    let (targets_len, text) = serialize_definitions_to_prometheus_config(definitions.clone());
    if targets_len > 0 {
        Ok(text)
    } else {
        Err((StatusCode::NOT_FOUND, "No targets found".to_string()))
    }
}
