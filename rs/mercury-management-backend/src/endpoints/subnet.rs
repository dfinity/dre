use super::*;
use crate::health;
use decentralization::network::TopologyManager;
use ic_base_types::PrincipalId;
use mercury_management_types::requests::{MembershipReplaceRequest, ReplaceTarget};
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
    .include_nodes(request.include.clone().unwrap_or_default());

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
                    .map(|s| !matches!(s, health::Status::Healthy))
                    .unwrap_or(true)
            })
            .map(|n| n.id)
            .collect::<Vec<_>>();
        let heal_count = unhealthy.len();
        if heal_count > 0 {
            change_request = change_request.remove(unhealthy)?;
            non_optimize_replaced_nodes += unhealthy.len();
            motivations.push(format!("replace {heal_count} unhealthy node(s)"));
        }
    }

    if let Some(optimize) = request.optimize {
        change_request = change_request.improve(optimize);
    }

    let change = change_request.evaluate()?;
    let optimize_replacements = change.removed().len() - non_optimize_replaced_nodes;
    if optimize_replacements > 0 {
        motivations.push(format!(
            "replace {optimize_replacements} node(s) to improve decentralization",
        ));
    }

    Ok(HttpResponse::Ok()
        .json(decentralization::SubnetChangeResponse::from(change).with_motivation(motivations.join(", "))))
}
