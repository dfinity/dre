use super::*;
use ic_management_types::Artifact;
use serde::Deserialize;

#[derive(Deserialize)]
struct ReleaseRequest {
    release_artifact: Artifact,
}

#[get("/releases/all")]
async fn releases_list_all(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    Ok(HttpResponse::Ok().json(registry.replica_releases()))
}

#[get("/release/retireable/{release_artifact}")]
async fn retireable(
    request: web::Path<ReleaseRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.retireable_versions(&request.release_artifact).await)
}

#[get("release/versions/blessed/{release_artifact}")]
async fn blessed(
    request: web::Path<ReleaseRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.blessed_versions(&request.release_artifact).await)
}

#[get("/release/versions/nns")]
async fn get_nns_replica_version(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    Ok(HttpResponse::Ok().json(registry.nns_replica_version().await))
}
