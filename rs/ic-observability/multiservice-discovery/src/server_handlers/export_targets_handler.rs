use super::Server;
use crate::{
    definition::{
        api_boundary_nodes_target_dtos_from_definitions, boundary_nodes_from_definitions,
        ic_node_target_dtos_from_definitions,
    },
    TargetFilterSpec,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use ic_types::{NodeId, PrincipalId};
use multiservice_discovery_shared::contracts::target::TargetDto;

pub(super) async fn export_targets(
    State(binding): State<Server>,
    filters: Query<TargetFilterSpec>,
) -> Result<Json<Vec<TargetDto>>, (StatusCode, String)> {
    let filters = filters.0;
    let definitions = binding.supervisor.definitions.lock().await;

    let ic_node_targets: Vec<TargetDto> = ic_node_target_dtos_from_definitions(&definitions, &filters);

    let boundary_nodes_targets = boundary_nodes_from_definitions(&definitions, &filters)
        .iter()
        .map(|(definition_name, bn)| TargetDto {
            name: bn.name.clone(),
            node_id: NodeId::from(PrincipalId::new_anonymous()),
            jobs: vec![bn.job_type],
            custom_labels: bn.custom_labels.clone(),
            targets: bn.targets.clone(),
            dc_id: "".to_string(),
            ic_name: definition_name.to_owned(),
            node_provider_id: PrincipalId::new_anonymous(),
            operator_id: PrincipalId::new_anonymous(),
            subnet_id: None,
            // These are old boundary nodes which are not the same as API boundary nodes
            // with time these should become api boundary nodes
            is_api_bn: false,
        })
        .collect();

    let api_boundary_nodes: Vec<TargetDto> = api_boundary_nodes_target_dtos_from_definitions(&definitions, &filters);

    let total_targets = [ic_node_targets, boundary_nodes_targets, api_boundary_nodes].concat();

    if !total_targets.is_empty() {
        Ok(Json(total_targets))
    } else {
        Err((StatusCode::NOT_FOUND, "No targets found".to_string()))
    }
}
