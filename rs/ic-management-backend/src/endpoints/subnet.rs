use super::*;
use crate::subnets::get_proposed_subnet_changes;
use ic_base_types::PrincipalId;
use ic_management_types::Node;
use serde::Deserialize;
use std::collections::BTreeMap;

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
            let registry_nodes: BTreeMap<PrincipalId, Node> = registry.read().await.nodes();

            get_proposed_subnet_changes(&registry_nodes, subnet)
                .map_err(actix_web::error::ErrorBadRequest)
                .map(|r| HttpResponse::Ok().json(r))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("failed to fetch subnets: {}", e))),
    }
}
