use super::*;
use crate::health::HealthStatusQuerier;
use crate::{health, subnets::get_proposed_subnet_changes};
use decentralization::network::{SubnetQueryBy, TopologyManager};
use ic_base_types::PrincipalId;
use ic_management_types::requests::{
    MembershipReplaceRequest, ReplaceTarget, SubnetCreateRequest, SubnetResizeRequest,
};
use ic_management_types::Node;
use log::warn;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct SubnetRequest {
    subnet: PrincipalId,
}

#[get("/subnet/{subnet}/pending_action")]
async fn pending_action(
    request: web::Path<SubnetRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    match registry.read().await.subnets_with_proposals().await {
        Ok(subnets) => {
            if let Some(subnet) = subnets.get(&request.subnet) {
                Ok(HttpResponse::Ok().json(&subnet.proposal))
            } else {
                Err(actix_web::error::ErrorNotFound(anyhow::format_err!(
                    "subnet {} not found",
                    request.subnet
                )))
            }
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!(
            "failed to fetch subnets: {}",
            e
        ))),
    }
}

#[get("/subnet/{subnet}/change_preview")]
async fn change_preview(
    request: web::Path<SubnetRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    match registry.read().await.subnets_with_proposals().await {
        Ok(subnets) => {
            let subnet = subnets.get(&request.subnet).ok_or_else(|| {
                actix_web::error::ErrorNotFound(anyhow::format_err!("subnet {} not found", request.subnet))
            })?;
            let registry_nodes: BTreeMap<PrincipalId, Node> = registry.read().await.nodes();

            get_proposed_subnet_changes(&registry_nodes, subnet)
                .map_err(actix_web::error::ErrorBadRequest)
                .map(|r| HttpResponse::Ok().json(r))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!(
            "failed to fetch subnets: {}",
            e
        ))),
    }
}

/// Simulates replacement of nodes in a subnet.
/// There are multiple ways to replace nodes. For instance:
///    1. Setting `heal` to `true` in the request to replace unhealthy nodes
///    2. Replace `optimize` nodes to optimize subnet decentralization.
///    3. Explicitly add or remove nodes from the subnet specifying their
/// Principals.
///
/// All nodes in the request must belong to exactly one subnet.
#[post("/subnet/membership/replace")]
async fn replace(
    request: web::Json<MembershipReplaceRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let all_nodes = registry.nodes();

    let mut motivations: Vec<String> = vec![];

    info!("Received MembershipReplaceRequest: {}", request);

    let change_request = match &request.target {
        ReplaceTarget::Subnet(subnet) => registry.modify_subnet_nodes(SubnetQueryBy::SubnetId(*subnet)).await?,
        ReplaceTarget::Nodes {
            nodes: nodes_to_replace,
            motivation,
        } => {
            motivations.push(motivation.clone());
            let nodes_to_replace = nodes_to_replace
                .iter()
                .filter_map(|n| all_nodes.get(n))
                .map(decentralization::network::Node::from)
                .collect::<Vec<_>>();
            registry
                .modify_subnet_nodes(SubnetQueryBy::NodeList(nodes_to_replace))
                .await?
        }
    }
    .with_exclude_nodes(request.exclude.clone().unwrap_or_default())
    .with_only_nodes_that_have_features(request.only.clone())
    .with_include_nodes(request.include.clone().unwrap_or_default())
    .with_min_nakamoto_coefficients(request.min_nakamoto_coefficients.clone());

    let mut replacements_unhealthy: Vec<decentralization::network::Node> = Vec::new();
    if request.heal {
        let subnet = change_request.subnet();
        let health_client = health::HealthClient::new(registry.network());
        let healths = health_client
            .subnet(subnet.id)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;
        let unhealthy: Vec<decentralization::network::Node> = subnet
            .nodes
            .into_iter()
            .filter_map(|n| match healths.get(&n.id) {
                Some(health) => {
                    if *health == ic_management_types::Status::Healthy {
                        None
                    } else {
                        info!("Node {} is {:?}", n.id, health);
                        Some(n)
                    }
                }
                None => {
                    warn!("Node {} has no known health, assuming unhealthy", n.id);
                    Some(n)
                }
            })
            .collect::<Vec<_>>();

        if !unhealthy.is_empty() {
            // Do not check the health of the force-included nodes
            let unhealthy = unhealthy
                .into_iter()
                .filter(|n| !request.include.as_ref().unwrap_or(&vec![]).contains(&n.id))
                .collect::<Vec<_>>();
            replacements_unhealthy.extend(unhealthy);
        }
    }
    let req_replace_nodes = if let ReplaceTarget::Nodes {
        nodes: req_replace_node_ids,
        motivation: _,
    } = &request.target
    {
        let req_replace_nodes = req_replace_node_ids
            .iter()
            .filter_map(|n| all_nodes.get(n))
            .map(decentralization::network::Node::from)
            .collect::<Vec<_>>();
        replacements_unhealthy.retain(|n| !req_replace_node_ids.contains(&n.id));
        req_replace_nodes
    } else {
        vec![]
    };

    let num_unhealthy = replacements_unhealthy.len();
    if !replacements_unhealthy.is_empty() {
        let replace_target = if num_unhealthy == 1 { "node" } else { "nodes" };
        motivations.push(format!("replacing {num_unhealthy} unhealthy {replace_target}"));
    }
    // Optimize the requested number of nodes, and remove unhealthy nodes if there
    // are any
    let replacements = replacements_unhealthy.into_iter().chain(req_replace_nodes).collect();
    let change = change_request.optimize(request.optimize.unwrap_or(0), &replacements)?;
    let num_optimized = change.removed().len() - replacements.len();
    if num_optimized > 0 {
        let replace_target = if num_optimized == 1 { "node" } else { "nodes" };
        motivations.push(format!(
            "replacing {num_optimized} {replace_target} to improve subnet decentralization"
        ));
    }

    Ok(HttpResponse::Ok()
        .json(decentralization::SubnetChangeResponse::from(&change).with_motivation(motivations.join("; "))))
}

/// Simulates creation of a new subnet
#[post("/subnet/create")]
async fn create_subnet(
    registry: web::Data<Arc<RwLock<RegistryState>>>,
    request: web::Json<SubnetCreateRequest>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    println!(
        "Received a request to create a subnet of size {:?} and MinNakamotoCoefficients {}",
        request.size,
        serde_json::to_string(&request.min_nakamoto_coefficients).unwrap()
    );

    Ok(HttpResponse::Ok().json(decentralization::SubnetChangeResponse::from(
        &registry
            .create_subnet(
                request.size,
                request.min_nakamoto_coefficients.clone(),
                request.include.clone().unwrap_or_default(),
                request.exclude.clone().unwrap_or_default(),
                request.only.clone().unwrap_or_default(),
            )
            .await?,
    )))
}

/// Simulates resizing the subnet, i.e. adding or removing nodes to a subnet.
#[post("/subnet/membership/resize")]
async fn resize(
    request: web::Json<SubnetResizeRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;

    let change = registry
        .modify_subnet_nodes(SubnetQueryBy::SubnetId(request.subnet))
        .await?
        .with_exclude_nodes(request.exclude.clone().unwrap_or_default())
        .with_include_nodes(request.include.clone().unwrap_or_default())
        .with_only_nodes_that_have_features(request.only.clone().unwrap_or_default())
        .resize(request.add, request.remove)?;

    Ok(HttpResponse::Ok().json(decentralization::SubnetChangeResponse::from(&change)))
}
