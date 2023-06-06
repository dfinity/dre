use super::*;
use crate::health;
use decentralization::{network::TopologyManager, SubnetChangeResponse};
use ic_base_types::PrincipalId;
use ic_management_types::requests::{
    MembershipReplaceRequest, ReplaceTarget, SubnetCreateRequest, SubnetResizeRequest,
};
use serde::Deserialize;

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
                Err(error::ErrorNotFound(anyhow::format_err!(
                    "subnet {} not found",
                    request.subnet
                )))
            }
        }
        Err(e) => Err(error::ErrorInternalServerError(format!(
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
    let nodes = registry.read().await.nodes();
    match registry.read().await.subnets_with_proposals().await {
        Ok(subnets) => {
            if let Some(subnet) = subnets.get(&request.subnet) {
                if let Some(proposal) = &subnet.proposal {
                    let removed_nodes = subnet
                        .nodes
                        .iter()
                        .filter(|n| proposal.nodes.contains(&n.principal))
                        .map(|n| n.principal)
                        .collect::<Vec<_>>();
                    let change_request = registry
                        .read()
                        .await
                        .replace_subnet_nodes(&removed_nodes)
                        .await?
                        .with_custom_available_nodes(
                            nodes
                                .values()
                                .filter(|n| n.subnet.is_none() && proposal.nodes.contains(&n.principal))
                                .map(decentralization::network::Node::from)
                                .collect(),
                        );
                    let mut change = SubnetChangeResponse::from(&change_request.evaluate()?);
                    change.proposal_id = Some(proposal.id);
                    Ok(HttpResponse::Ok().json(change))
                } else {
                    Err(error::ErrorBadRequest(anyhow::format_err!(
                        "subnet {} does not have open membership change proposals",
                        request.subnet
                    )))
                }
            } else {
                Err(error::ErrorNotFound(anyhow::format_err!(
                    "subnet {} not found",
                    request.subnet
                )))
            }
        }
        Err(e) => Err(error::ErrorInternalServerError(format!(
            "failed to fetch subnets: {}",
            e
        ))),
    }
}

/// Simulates replacement of nodes in a subnet.
/// There are three different ways to replace nodes:
///    1. Setting `heal` to `true` in the request to replace unhealthy nodes on
/// the subnet.    2. Replace `n` amount of nodes to optimize the subnet
/// decentralization by specifying `optimize`.    3. Explicitly specifying nodes
/// in a subnet by specifying their Principals in the `nodes` field.
/// All three methods can be used at the same time.
///
/// Target subnet is selected by either specifying subnet id explicitly or by
/// specifying nodes in the same subnet. Specifying both, neither, or nodes in
/// different subnets, will result in an error.
#[post("/subnet/membership/replace")]
async fn replace(
    request: web::Json<MembershipReplaceRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;

    let mut motivations: Vec<String> = vec![];

    let mut change_request = match &request.target {
        ReplaceTarget::Subnet(subnet) => registry.modify_subnet_nodes(*subnet).await?,
        ReplaceTarget::Nodes { nodes, motivation } => {
            motivations.push(motivation.clone());
            registry.replace_subnet_nodes(nodes).await?
        }
    }
    .exclude_nodes(request.exclude.clone().unwrap_or_default())
    .include_nodes(request.include.clone().unwrap_or_default())
    .with_min_nakamoto_coefficients(request.min_nakamoto_coefficients.clone());

    let mut non_optimize_replaced_nodes = if let ReplaceTarget::Nodes { nodes, motivation: _ } = &request.target {
        nodes.len()
    } else {
        0
    };
    if request.heal {
        let subnet = change_request.subnet();
        let health_client = health::HealthClient::new(registry.network());
        let healths = health_client
            .subnet(subnet.id)
            .await
            .map_err(|_| error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;
        let unhealthy = &subnet
            .nodes
            .iter()
            .filter(|n| {
                healths
                    .get(&n.id)
                    // TODO: Add option to exclude degraded nodes from healing
                    .map(|s| !matches!(s, ic_management_types::Status::Healthy))
                    .unwrap_or(true)
            })
            .map(|n| n.id)
            .collect::<Vec<_>>();
        let heal_count = unhealthy.len();
        if heal_count > 0 {
            change_request = change_request.remove(unhealthy)?;
            non_optimize_replaced_nodes += unhealthy.len();
            let replace_target = if heal_count == 1 { "node" } else { "nodes" };
            motivations.push(format!("replacing {heal_count} unhealthy {replace_target}"));
        }
    }

    if let Some(optimize) = request.optimize {
        change_request = change_request.improve(optimize);
    }

    let change = change_request.evaluate()?;
    let optimize_replacements = change.removed().len() - non_optimize_replaced_nodes;
    if optimize_replacements > 0 {
        let replace_target = if optimize_replacements == 1 { "node" } else { "nodes" };
        motivations.push(format!(
            "replacing {optimize_replacements} {replace_target} to improve subnet decentralization",
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
            .create_subnet(request.size, request.min_nakamoto_coefficients.clone())
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
        .modify_subnet_nodes(request.subnet)
        .await?
        .exclude_nodes(request.exclude.clone().unwrap_or_default())
        .include_nodes(request.include.clone().unwrap_or_default())
        .resize(request.add, request.remove)?;

    Ok(HttpResponse::Ok().json(decentralization::SubnetChangeResponse::from(&change)))
}
