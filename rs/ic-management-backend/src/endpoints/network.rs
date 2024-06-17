use super::*;
use crate::subnets;
use decentralization::network::NetworkHealRequest;
use ic_management_types::requests::HealRequest;

#[post("/network/heal")]
async fn heal(request: web::Json<HealRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    let nodes_health = health_client
        .nodes()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;
    let subnets: BTreeMap<PrincipalId, ic_management_types::Subnet> = registry.subnets();
    let unhealthy_subnets: BTreeMap<PrincipalId, Vec<ic_management_types::Node>> = subnets::unhealthy_with_nodes(&subnets, nodes_health).await;

    let subnets_change_response =
        NetworkHealRequest{ subnets, unhealthy_subnets}.heal(registry.available_nodes().await?, request.max_replacable_nodes_per_sub)?;

    Ok(HttpResponse::Ok().json(decentralization::HealResponse { subnets_change_response }))
}
