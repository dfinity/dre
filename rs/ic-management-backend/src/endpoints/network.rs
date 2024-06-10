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
            .with_exclude_nodes(already_added.iter().map(|n: &PrincipalId| n.to_string()).collect());

        let subnet_change = if let Some(max_replacable_nodes_per_sub) = request.max_replacable_nodes_per_sub {
            let optimize_limit = max_replacable_nodes_per_sub - unhealthy_nodes.len();
            change.optimize_with_limit(optimize_limit, &unhealthy_nodes)?
        } else {
            let not_optimized = change.optimize(0, &unhealthy_nodes)?;
            decentralization::SubnetChangeResponse::from(&not_optimized)
        };

        already_added.extend(subnet_change.added.clone());
        subnets_changed.push(subnet_change);
    }

    let heal_response = decentralization::HealResponse {
        subnets_change_response: subnets_changed,
    };
    Ok(HttpResponse::Ok().json(heal_response))
}
