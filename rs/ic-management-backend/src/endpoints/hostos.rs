use ic_management_types::requests::HostosRolloutRequest;

use super::*;
use crate::hostos_rollout;

#[post("/hostos/rollout_nodes")]
async fn rollout_nodes(
    request: web::Json<HostosRolloutRequest>,
    registry: web::Data<Arc<RwLock<RegistryState>>>,
) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let proposal_agent = proposal::ProposalAgent::new(registry.nns_url());
    let network = registry.network();

    let hostos_rollout = hostos_rollout::HostosRollout::new(
        registry.nodes(),
        registry.subnets(),
        network,
        proposal_agent,
        &request.version,
    );

    response_from_result(hostos_rollout.execute(request.node_group).await)
}
