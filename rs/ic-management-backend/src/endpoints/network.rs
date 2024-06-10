use super::*;
use crate::health;
use crate::health::HealthStatusQuerier;
use decentralization::network::{SubnetQueryBy, TopologyManager};
use ic_management_types::requests::HealRequest;
use log::warn;
use std::collections::{BTreeMap, HashSet};

#[post("/network/heal")]
async fn heal(request: web::Json<HealRequest>, registry: web::Data<Arc<RwLock<RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    let healths = health_client
        .nodes()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("failed to fetch subnet health".to_string()))?;

    let unhealthy_subnets = registry
        .subnets()
        .into_iter()
        .filter_map(|(_, subnet)| {
            let unhealthy = subnet
                .nodes
                .into_iter()
                .filter_map(|n| match healths.get(&n.principal) {
                    Some(health) => {
                        if *health == ic_management_types::Status::Healthy {
                            None
                        } else {
                            info!("Node {} is {:?}", n.principal, health);
                            Some(n)
                        }
                    }
                    None => {
                        warn!("Node {} has no known health, assuming unhealthy", n.principal);
                        Some(n)
                    }
                })
                .map(|n| decentralization::network::Node::from(&n))
                .collect::<Vec<_>>();

            if !unhealthy.is_empty() {
                if let Some(x) = request.max_replacable_nodes_per_sub {
                    if unhealthy.len() > x {
                        warn!(
                            "Subnet {} has {} unhealthy nodes\nMax replacable nodes is {} skipping...",
                            subnet.principal,
                            unhealthy.len(),
                            x
                        );
                        return None;
                    }
                }
                return Some((subnet.principal, unhealthy));
            } else {
                None
            }
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
            let optimize_buffer = max_replacable_nodes_per_sub - unhealthy_nodes.len();

            change.optimize_with_buffer(optimize_buffer, &unhealthy_nodes)?
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
