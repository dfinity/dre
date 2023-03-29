use super::*;

#[get("/release/retireable")]
async fn retireable(registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.retireable_versions().await)
}
