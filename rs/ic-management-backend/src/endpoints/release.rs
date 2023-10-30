use super::*;

#[get("/releases/all")]
async fn releases_list_all(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    Ok(HttpResponse::Ok().json(registry.replica_releases()))
}

#[get("/release/retireable")]
async fn retireable(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.retireable_versions().await)
}

#[get("/release/versions/nns")]
async fn get_nns_replica_version(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    Ok(HttpResponse::Ok().json(registry.nns_replica_version().await))
}
