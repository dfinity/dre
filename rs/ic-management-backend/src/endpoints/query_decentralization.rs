use super::*;
use decentralization::network::{DecentralizedSubnet, SubnetChange};
use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use ic_management_types::MinNakamotoCoefficients;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct SubnetRequest {
    subnet: PrincipalId,
}

#[derive(Serialize, Deserialize)]
struct DecentralizedSubnetResponse {
    id: PrincipalId,
    message: String,
    run_log: String,
    nakamoto: decentralization::nakamoto::NakamotoScore,
}

/// Get the decentralization coefficients for a subnet
#[get("/decentralization/subnet/{subnet}")]
pub(crate) async fn decentralization_subnet_query(
    request: web::Path<SubnetRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    get_decentralization_analysis(registry, Some(request.subnet), None, None, None).await
}

#[derive(Deserialize)]
struct SubnetWhatIfRequest {
    subnet: Option<PrincipalId>,
    nodes_to_add: Option<Vec<PrincipalId>>,
    nodes_to_remove: Option<Vec<PrincipalId>>,
    min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
}

/// Get the decentralization coefficients for a subnet
#[get("/decentralization/whatif")]
pub(crate) async fn decentralization_whatif_query(
    request: web::Json<SubnetWhatIfRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    get_decentralization_analysis(
        registry,
        request.subnet,
        request.nodes_to_add.clone(),
        request.nodes_to_remove.clone(),
        request.min_nakamoto_coefficients.clone(),
    )
    .await
}

async fn get_decentralization_analysis(
    registry: web::Data<Arc<RwLock<RegistryState>>>,
    subnet: Option<PrincipalId>,
    node_ids_to_add: Option<Vec<PrincipalId>>,
    node_ids_to_remove: Option<Vec<PrincipalId>>,
    min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
) -> Result<HttpResponse, Error> {
    let subnets = registry.read().await.subnets();
    let registry_nodes = registry.read().await.nodes();

    let original_subnet = subnet
        .map(|subnet_id| match subnets.get(&subnet_id) {
            Some(subnet) => DecentralizedSubnet {
                id: subnet_id,
                nodes: subnet.nodes.iter().map(decentralization::network::Node::from).collect(),
                added_nodes_desc: Vec::new(),
                removed_nodes_desc: Vec::new(),
                min_nakamoto_coefficients: min_nakamoto_coefficients.clone(),
                comment: None,
                run_log: Vec::new(),
            },
            None => DecentralizedSubnet {
                id: PrincipalId::new_subnet_test_id(0),
                nodes: Vec::new(),
                added_nodes_desc: Vec::new(),
                removed_nodes_desc: Vec::new(),
                min_nakamoto_coefficients: min_nakamoto_coefficients.clone(),
                comment: None,
                run_log: Vec::new(),
            },
        })
        .unwrap_or_else(|| DecentralizedSubnet {
            id: PrincipalId::new_subnet_test_id(0),
            nodes: Vec::new(),
            added_nodes_desc: Vec::new(),
            removed_nodes_desc: Vec::new(),
            min_nakamoto_coefficients: min_nakamoto_coefficients.clone(),
            comment: None,
            run_log: Vec::new(),
        });

    let nodes_to_remove = node_ids_to_remove.map(|node_ids_to_remove| {
        node_ids_to_remove
            .iter()
            .filter_map(|n| registry_nodes.get(n))
            .map(|n| (decentralization::network::Node::from(n), "".to_string()))
            .collect::<Vec<_>>()
    });
    let updated_subnet = match &nodes_to_remove {
        Some(nodes_to_remove) => original_subnet.without_nodes(nodes_to_remove.clone())?,
        None => original_subnet.clone(),
    };

    let updated_subnet = match &node_ids_to_add {
        Some(nodes_to_add) => {
            let nodes_to_add = nodes_to_add
                .iter()
                .map(|n| (decentralization::network::Node::from(&registry_nodes[n]), "added".to_string()))
                .collect();
            updated_subnet.with_nodes(nodes_to_add)
        }
        None => updated_subnet,
    };

    let subnet_change = SubnetChange {
        id: original_subnet.id,
        old_nodes: original_subnet.nodes,
        new_nodes: updated_subnet.nodes.clone(),
        removed_nodes_desc: updated_subnet.removed_nodes_desc.clone(),
        added_nodes_desc: updated_subnet.added_nodes_desc.clone(),
        min_nakamoto_coefficients: updated_subnet.min_nakamoto_coefficients.clone(),
        comment: updated_subnet.comment.clone(),
        run_log: updated_subnet.run_log.clone(),
    };

    let response = DecentralizedSubnetResponse {
        id: subnet.unwrap_or_else(|| PrincipalId::new_subnet_test_id(0)),
        message: format!("{}", SubnetChangeResponse::from(&subnet_change)),
        nakamoto: updated_subnet.nakamoto_score(),
        run_log: subnet_change.run_log.join("\n"),
    };
    Ok(HttpResponse::Ok().json(&response))
}
