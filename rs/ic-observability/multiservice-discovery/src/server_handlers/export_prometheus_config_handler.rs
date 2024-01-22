use super::WebResult;
use crate::definition::Definition;
use multiservice_discovery_shared::builders::prometheus_config_structure::{map_target_group, PrometheusStaticConfig};
use multiservice_discovery_shared::contracts::target::{map_to_target_dto, TargetDto};
use service_discovery::{
    job_types::{JobType, NodeOS},
    IcServiceDiscovery,
};
use slog::Logger;
use std::{collections::BTreeMap, collections::BTreeSet, sync::Arc};
use tokio::sync::Mutex;
use warp::reply::Reply;

pub struct ExportDefinitionConfigBinding {
    pub definitions: Arc<Mutex<Vec<Definition>>>,
    pub log: Logger,
}

pub async fn export_prometheus_config(binding: ExportDefinitionConfigBinding) -> WebResult<impl Reply> {
    let definitions = binding.definitions.lock().await;

    let mut total_targets: Vec<TargetDto> = vec![];

    for def in definitions.iter() {
        for job_type in JobType::all_for_ic_nodes() {
            let targets = match def.ic_discovery.get_target_groups(job_type, binding.log.clone()) {
                Ok(targets) => targets,
                Err(_) => continue,
            };

            targets.iter().for_each(|target_group| {
                if let Some(target) = total_targets.iter_mut().find(|t| t.node_id == target_group.node_id) {
                    target.jobs.push(job_type);
                } else {
                    total_targets.push(map_to_target_dto(
                        target_group,
                        job_type,
                        BTreeMap::new(),
                        target_group.node_id.to_string(),
                        def.name.clone(),
                    ));
                }
            });
        }
    }

    let mut total_set = map_target_group(total_targets.into_iter().collect());

    total_set.extend(
        definitions
            .iter()
            .flat_map(|def| {
                def.boundary_nodes.iter().filter_map(|bn| {
                    // Boundary nodes do not have metrics-proxy.
                    if let JobType::MetricsProxy(_) = bn.job_type {
                        return None;
                    }
                    // If this boundary node is under the test environment,
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
                                ("ic", def.name.clone()),
                                ("name", bn.name.clone()),
                                ("job", bn.job_type.to_string()),
                            ])
                            .into_iter()
                            .map(|k| (k.0.to_string(), k.1))
                            .chain(bn.custom_labels.clone())
                            .collect::<BTreeMap<_, _>>()
                        },
                    })
                })
            })
            .collect::<BTreeSet<PrometheusStaticConfig>>(),
    );

    Ok(warp::reply::with_status(
        serde_json::to_string_pretty(&total_set).unwrap(),
        if !total_set.is_empty() {
            warp::http::StatusCode::OK
        } else {
            warp::http::StatusCode::NOT_FOUND
        },
    ))
}
