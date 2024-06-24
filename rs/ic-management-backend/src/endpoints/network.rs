use super::*;
use crate::subnets;
use decentralization::network::{DecentralizedSubnet, NetworkHealRequest, NetworkHealSubnets, Node};
use ic_management_types::{requests::HealRequest, NetworkError};
use itertools::Itertools;

#[post("/network/heal")]
pub(crate) async fn heal(request: web::Json<HealRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    let nodes_health = health_client
        .nodes()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;
    let subnets: BTreeMap<PrincipalId, ic_management_types::Subnet> = registry.subnets();
    let unhealthy_subnets: BTreeMap<PrincipalId, Vec<ic_management_types::Node>> = subnets::unhealthy_with_nodes(&subnets, nodes_health).await;

    let subnets_to_heal = unhealthy_subnets
        .iter()
        .flat_map(|(id, unhealthy_nodes)| {
            let unhealthy_nodes = unhealthy_nodes.iter().map(Node::from).collect::<Vec<_>>();
            let unhealthy_subnet = subnets.get(id).ok_or(NetworkError::SubnetNotFound(*id))?;

            Ok::<NetworkHealSubnets, NetworkError>(NetworkHealSubnets {
                name: unhealthy_subnet.metadata.name.clone(),
                decentralized_subnet: DecentralizedSubnet::from(unhealthy_subnet),
                unhealthy_nodes,
            })
        })
        .collect_vec();

    let subnets_change_response =
        NetworkHealRequest::new(subnets_to_heal).heal_and_optimize(registry.available_nodes().await?, request.max_replaceable_nodes_per_sub)?;

    Ok(HttpResponse::Ok().json(decentralization::HealResponse { subnets_change_response }))
}
