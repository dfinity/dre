use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use decentralization::{
    network::{DashboardAgent, DecentralizationQuerier, InternalDashboardAgent, SubnetQuerier, SubnetsManager},
    OptimizeQuery, ReplaceRequest, SubnetChangeResponse, SubnetCreateRequest,
};
use ic_base_types::PrincipalId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static IC_API_NODES_URL: &str = "https://ic-api.internetcomputer.org/api/v3/nodes";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subnet_agent = Arc::new(RwLock::new(DashboardAgent::new(IC_API_NODES_URL.to_string())));
    HttpServer::new(move || {
        let subnet_clone = subnet_agent.clone();
        App::new()
            .app_data(web::Data::new(subnet_clone))
            .service(smoke)
            .service(get_subnets)
            .service(get_decentralization)
            .service(replace_nodes)
            .service(optimize_subnet)
            .service(create_subnet)
    })
    .bind(("0.0.0.0", 8081))?
    .run()
    .await
}

#[get("/smoke")]
async fn smoke() -> impl Responder {
    HttpResponse::Ok().json(serde_json::to_string("smoke test passed!").unwrap())
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SubnetQuery {
    id: PrincipalId,
}

#[get("/subnets")]
async fn get_subnets(
    querier: web::Data<Arc<RwLock<DashboardAgent>>>,
    query: web::Query<SubnetQuery>,
) -> impl Responder {
    return HttpResponse::Ok().json(querier.read().await.subnet(&query.id).await);
}

#[get("/decentralization")]
async fn get_decentralization(
    querier: web::Data<Arc<RwLock<DashboardAgent>>>,
    query: web::Query<SubnetQuery>,
) -> impl Responder {
    return HttpResponse::Ok().json(querier.read().await.decentralization(query.id).await);
}

#[post("/create")]
async fn create_subnet(
    agent: InternalDashboardAgent,
    request: web::Json<SubnetCreateRequest>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(
        SubnetsManager::new(agent)
            .create(request.size)
            .await?
            .into_iter()
            .map(|x| x.id)
            .collect::<Vec<PrincipalId>>(),
    ))
}

#[derive(Deserialize)]
pub struct SubnetRequest {
    pub subnet: PrincipalId,
}

#[post("/replace")]
async fn replace_nodes(info: web::Json<ReplaceRequest>) -> Result<HttpResponse, Error> {
    let agent = InternalDashboardAgent::new(dashboard_backend_url());
    let subnet = agent.subnet_of_nodes(&info.nodes).await?;
    let replaced = SubnetsManager::new(agent)
        .subnet(subnet.id)
        .await?
        .replace(&info.nodes)?;

    Ok(HttpResponse::Ok().json(SubnetChangeResponse::from(replaced)))
}

#[post("/{subnet}/optimize")]
async fn optimize_subnet(
    request: web::Path<SubnetRequest>,
    query: web::Query<OptimizeQuery>,
) -> Result<HttpResponse, Error> {
    const DEFAULT_MAX_REPLACEMENTS: usize = 2;
    let optimized = SubnetsManager::new(InternalDashboardAgent::new(dashboard_backend_url()))
        .subnet(request.subnet)
        .await?
        .optimize(query.max_replacements.unwrap_or(DEFAULT_MAX_REPLACEMENTS))?;

    Ok(HttpResponse::Ok().json(SubnetChangeResponse::from(optimized)))
}

fn dashboard_backend_url() -> String {
    std::env::var("DASHBOARD_BACKEND_URL")
        .unwrap_or_else(|_| "https://dashboard.mercury.dfinity.systems/api/proxy/registry".to_string())
}
