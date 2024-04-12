use crate::TargetFilterSpec;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use ic_types::{NodeId, PrincipalId};
use multiservice_discovery_shared::contracts::target::{map_to_target_dto, TargetDto};
use service_discovery::job_types::{JobType, NodeOS};
use std::collections::BTreeMap;

use super::Server;

pub(super) async fn export_targets(
    State(binding): State<Server>,
    filters: Query<TargetFilterSpec>,
) -> Result<Json<Vec<TargetDto>>, (StatusCode, String)> {
    let filters = filters.0;
    let definitions = binding.supervisor.definitions.lock().await;

    let mut ic_node_targets: Vec<TargetDto> = vec![];

    for (ic_name, def) in definitions.iter() {
        if filters.matches_ic(ic_name) {
            for job_type in JobType::all_for_ic_nodes() {
                let target_groups = match def.get_target_groups(job_type) {
                    Ok(target_groups) => target_groups,
                    Err(_) => continue,
                };

                target_groups.iter().for_each(|target_group| {
                    if let Some(target) = ic_node_targets.iter_mut().find(|t| t.node_id == target_group.node_id) {
                        target.jobs.push(job_type);
                    } else {
                        let target = map_to_target_dto(
                            target_group,
                            job_type,
                            BTreeMap::new(),
                            target_group.node_id.to_string(),
                            def.name(),
                        );
                        if filters.matches_ic_node(&target) {
                            ic_node_targets.push(target)
                        };
                    }
                });
            }
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
                if !filters.matches_boundary_node(bn) {
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

    if !total_targets.is_empty() {
        Ok(Json(total_targets))
    } else {
        Err((StatusCode::NOT_FOUND, "No targets found".to_string()))
    }
}
