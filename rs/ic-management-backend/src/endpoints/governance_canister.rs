use super::*;
use ic_canisters::governance::governance_canister_version;

#[get("/canisters/governance/version")]
async fn governance_canister_version_endpoint(
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let g = governance_canister_version(registry.network().get_nns_urls().clone()).await;
    response_from_result(g)
}
