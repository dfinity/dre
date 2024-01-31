use super::WebResult;
use crate::definition::DefinitionsSupervisor;
use ic_types::{NodeId, PrincipalId};
use multiservice_discovery_shared::contracts::target::{map_to_target_dto, TargetDto};
use service_discovery::job_types::{JobType, NodeOS};
use std::collections::BTreeMap;
use warp::reply::Reply;

#[derive(Clone)]
pub(super) struct ExportTargetsBinding {
    pub(crate) supervisor: DefinitionsSupervisor,
}

pub(super) async fn export_targets(binding: ExportTargetsBinding) -> WebResult<impl Reply> {
    let definitions = binding.supervisor.definitions.lock().await;

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
                Some(TargetDto {
                    name: bn.name.clone(),
                    node_id: NodeId::from(PrincipalId::new_anonymous()),
                    jobs: vec![bn.job_type],
                    custom_labels: bn.custom_labels.clone(),
                    targets: bn.targets.clone(),
                    dc_id: "".to_string(),
                    ic_name: def.name(),
                    node_provider_id: PrincipalId::new_anonymous(),
                    operator_id: PrincipalId::new_anonymous(),
                    subnet_id: None,
                })
            })
        })
        .collect::<Vec<TargetDto>>();

    let total_targets = [ic_node_targets, boundary_nodes_targets].concat();

    Ok(warp::reply::with_status(
        serde_json::to_string_pretty(&total_targets).unwrap(),
        if !total_targets.is_empty() {
            warp::http::StatusCode::OK
        } else {
            warp::http::StatusCode::NOT_FOUND
        },
    ))
}
