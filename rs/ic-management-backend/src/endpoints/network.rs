use super::*;
use crate::subnets;
use decentralization::network::{Node, SubnetQueryBy, TopologyManager};
use ic_management_types::requests::HealRequest;
use log::warn;
use std::collections::HashSet;

#[post("/network/heal")]
async fn heal(request: web::Json<HealRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let mut already_added = HashSet::new();
    let mut subnets_changed = Vec::new();

    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    let nodes_health = health_client
        .nodes()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;

    let unhealthy_subnets = subnets::unhealthy_with_nodes(&registry.subnets(), nodes_health)
        .await
        .into_iter()
        .filter_map(|(subnet_id, n)| {
            if let Some(max_replacable_nodes) = request.max_replacable_nodes_per_sub {
                if n.len() > max_replacable_nodes {
                    warn!(
                        "Subnet {} has {} unhealthy nodes\nMax replacable nodes is {} skipping...",
                        subnet_id,
                        n.len(),
                        max_replacable_nodes
                    );
                    return None;
                }
            }
            Some((subnet_id, n))
        })
        .collect::<BTreeMap<_, _>>();

    for (id, unhealthy_nodes) in unhealthy_subnets {
        let decentralized_unhealthy_nodes: Vec<Node> = unhealthy_nodes.iter().map(decentralization::network::Node::from).collect::<Vec<_>>();
        let unhealthy_nodes_len = unhealthy_nodes.len();
        let optimize_limit = request.max_replacable_nodes_per_sub.unwrap_or(unhealthy_nodes_len) - unhealthy_nodes_len;

        let optimized = registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(id))
            .await?
            .with_exclude_nodes(already_added.iter().cloned().collect::<Vec<_>>())
            .optimize(optimize_limit, &decentralized_unhealthy_nodes)?;

        already_added.extend(optimized.added().iter().map(|n| n.id.to_string()).collect::<Vec<_>>());
        subnets_changed.push(decentralization::SubnetChangeResponse::from(&optimized));
    }

    let response = decentralization::HealResponse {
        subnets_change_response: subnets_changed,
    };
    Ok(HttpResponse::Ok().json(response))
}
