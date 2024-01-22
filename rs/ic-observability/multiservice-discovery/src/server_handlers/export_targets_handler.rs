use super::WebResult;
use crate::definition::Definition;
use ic_types::{NodeId, PrincipalId};
use multiservice_discovery_shared::contracts::target::{map_to_target_dto, TargetDto};
use service_discovery::{
    job_types::{JobType, NodeOS},
    IcServiceDiscovery,
};
use slog::Logger;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;
use warp::reply::Reply;

pub struct ExportTargetsBinding {
    pub definitions: Arc<Mutex<Vec<Definition>>>,
    pub log: Logger,
}

pub async fn export_targets(binding: ExportTargetsBinding) -> WebResult<impl Reply> {
    let definitions = binding.definitions.lock().await;

    let mut total_targets: Vec<TargetDto> = vec![];

    for def in definitions.iter() {
        for job_type in JobType::all() {
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

    total_targets.extend(
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
                    Some(TargetDto {
                        name: bn.name.clone(),
                        node_id: NodeId::from(PrincipalId::new_anonymous()),
                        jobs: vec![bn.job_type],
                        custom_labels: bn.custom_labels.clone(),
                        targets: bn.targets.clone(),
                        dc_id: "".to_string(),
                        ic_name: def.name.clone(),
                        node_provider_id: PrincipalId::new_anonymous(),
                        operator_id: PrincipalId::new_anonymous(),
                        subnet_id: None,
                    })
                })
            })
            .collect::<Vec<TargetDto>>(),
    );

    Ok(warp::reply::with_status(
        serde_json::to_string_pretty(&total_targets).unwrap(),
        if !total_targets.is_empty() {
            warp::http::StatusCode::OK
        } else {
            warp::http::StatusCode::NOT_FOUND
        },
    ))
}
