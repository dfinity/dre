use super::super::config::nns_nodes_urls;
use super::*;
use ic_canisters::governance_canister_version;

#[get("/canisters/governance/version")]
async fn governance_canister_version_endpoint() -> Result<HttpResponse, Error> {
    let u = nns_nodes_urls()[0].clone();
    let g = governance_canister_version(u).await;
    response_from_result(g)
}
