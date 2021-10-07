use crate::types::*;
use reqwest::Client;

#[allow(dead_code)]
async fn add_recommended_nodes_to_subnet(
    subnet: Subnet,
    node_count: i32,
    client: &Client,
    url: &str,
) {
    let body = DecentralizedNodeQuery {
        removals: None,
        subnet: subnet.id.clone(),
        node_count,
    };
    let best_nodes = get_decentralized_nodes(url, client, body)
        .await
        .expect("Unable to get nodes from backend")
        .nodes;
    println!("The current best nodes to add are {:?}", best_nodes);
}

#[allow(dead_code)]
pub async fn remove_dead_nodes_from_subnet(
    subnet: Subnet,
    url: &str,
    client: &Client,
    // dryrun: DryRun,
) -> Result<(), Error> {
    println!("Not implemented yet (remove_nodes)");
    let nodes_to_remove = get_dead_nodes(subnet.clone(), url, client).await?;
    let assumed_removed = nodes_to_remove.nodes.clone();
    let node_count = assumed_removed.len() as i32;
    let body = DecentralizedNodeQuery {
        subnet: subnet.id.clone(),
        removals: Some(assumed_removed.clone()),
        node_count,
    };
    let best_added_nodes = get_decentralized_nodes(url, client, body).await?.nodes;
    println!(
        "The current dead nodes are {:?}, and the nodes that we would like to add are {:?}",
        assumed_removed, best_added_nodes
    );
    Ok(())
}

pub async fn get_decentralized_nodes(
    url: &str,
    client: &Client,
    params: DecentralizedNodeQuery,
) -> Result<BestNodesResponse, anyhow::Error> {
    let resp = client
        .post(url)
        .json(&params)
        .send()
        .await?
        .json::<BestNodesResponse>()
        .await?;
    Ok(resp)
}

pub async fn get_dead_nodes(
    subnet: Subnet,
    url: &str,
    client: &Client,
) -> Result<NodesToRemoveResponse, anyhow::Error> {
    let resp = client
        .get(url)
        .query(&[("subnet", subnet.id)])
        .send()
        .await?
        .json::<NodesToRemoveResponse>()
        .await?;
    Ok(resp)
}
