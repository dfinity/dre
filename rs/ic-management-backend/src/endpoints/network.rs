use super::*;
use crate::subnets;
use decentralization::network::{SubnetQueryBy, TopologyManager};
use ic_management_types::requests::HealRequest;
use log::warn;
use std::collections::HashSet;

#[post("/network/heal")]
async fn heal(request: web::Json<HealRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let unhealthy_subnets = subnets::get_unhealthy(&registry)
        .await        
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?
        .into_iter()
        .filter_map(|(subnet_id, n)| {
            if let Some(x) = request.max_replacable_nodes_per_sub {
                if n.len() > x {
                    warn!(
                        "Subnet {} has {} unhealthy nodes\nMax replacable nodes is {} skipping...",
                        subnet_id,
                        n.len(),
                        x
                    );
                    return None;
                }
            }
            return Some((subnet_id, n));
        })
        .collect::<BTreeMap<_, _>>();

    let mut already_added = HashSet::new();
    let mut subnets_changed = Vec::new();

    for (id, unhealthy_nodes) in unhealthy_subnets {
        let change = registry
            .modify_subnet_nodes(SubnetQueryBy::SubnetId(id))
            .await?
            .with_exclude_nodes(already_added.iter().cloned().collect::<Vec<_>>());

        let optimize_limit = request.max_replacable_nodes_per_sub.unwrap_or(unhealthy_nodes.len()) - unhealthy_nodes.len();
        let optimized = change.optimize(optimize_limit, &unhealthy_nodes)?;

        already_added.extend(optimized.added().iter().map(|n| n.id.to_string()).collect::<Vec<_>>());
        subnets_changed.push(decentralization::SubnetChangeResponse::from(&optimized));
    }

    let heal_response = decentralization::HealResponse {
        subnets_change_response: subnets_changed,
    };
    Ok(HttpResponse::Ok().json(heal_response))
}
