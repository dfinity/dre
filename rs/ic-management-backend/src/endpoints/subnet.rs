use super::*;
use crate::health::HealthClient;
use crate::subnets::get_proposed_subnet_changes;
use ic_base_types::PrincipalId;
use ic_management_types::Node;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize)]
struct SubnetRequest {
    subnet: PrincipalId,
}

#[get("/subnet/{subnet}/change_preview")]
pub(crate) async fn change_preview(
    request: web::Path<SubnetRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    match registry.read().await.subnets_with_proposals().await {
        Ok(subnets) => {
            let subnet = subnets
                .get(&request.subnet)
                .ok_or_else(|| actix_web::error::ErrorNotFound(anyhow::format_err!("subnet {} not found", request.subnet)))?;
            let registry_nodes: IndexMap<PrincipalId, Node> = registry.read().await.nodes();
            let health_of_nodes = HealthClient::new(registry.read().await.network(), None, false)
                .nodes()
                .await
                .unwrap_or_default();

            get_proposed_subnet_changes(&registry_nodes, subnet, &health_of_nodes)
                .map_err(actix_web::error::ErrorBadRequest)
                .map(|r| HttpResponse::Ok().json(r))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("failed to fetch subnets: {}", e))),
    }
}
