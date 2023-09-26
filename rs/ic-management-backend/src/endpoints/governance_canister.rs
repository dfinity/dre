use super::*;
use crate::config::{get_nns_url_vec_from_target_network, target_network};
use ic_canisters::governance_canister_version;

#[get("/canisters/governance/version")]
async fn governance_canister_version_endpoint() -> Result<HttpResponse, Error> {
    let network = target_network();
    let u = get_nns_url_vec_from_target_network(&network)[0].clone();
    let g = governance_canister_version(u).await;
    response_from_result(g)
}
