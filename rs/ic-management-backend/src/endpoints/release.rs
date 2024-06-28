use super::*;
use ic_management_types::Artifact;
use serde::Deserialize;

#[derive(Deserialize)]
struct ReleaseRequest {
    release_artifact: Artifact,
}

#[get("/releases/all")]
pub(crate) async fn releases_list_all(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    Ok(HttpResponse::Ok().json(registry.replica_releases()))
}

#[get("release/versions/blessed/{release_artifact}")]
pub(crate) async fn blessed(request: web::Path<ReleaseRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.blessed_versions(&request.release_artifact).await)
}
