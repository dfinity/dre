use confy::load;
use serde::{Serialize, Deserialize};
use clap::{App, load_yaml};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use anyhow::{anyhow, Error};
use std::fmt::Display;
mod types;
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct OperatorConfig {
    hsm_pin: String,
    hsm_slot: String,
    hsm_key_id: String,
    neuron_id: String,
    proposal_url: String,
    ic_admin_cmd: String,
    backend_url: String
}

fn main() {
    let client = reqwest::Client::new();
    let yaml = load_yaml!("cli.yaml");
    let args = App::from(yaml).get_matches();
    println!{"{:?}", args};
    let cfg: OperatorConfig = confy::load_path("management_config.toml").unwrap();
    println!("Hello, world!");
    let subcommand = match args.subcommand() {
        ( "add-nodes", Some(v)) => { 
            match v.value_of("subnet") {
                Some(s) => { return add_nodes_to_subnet(s.to_string())},
                None => { println!("Need subnet to perform add-nodes")}
            }
         },
        ( "remove-dead-nodes", Some(v)) => { println!("remove-dead-nodes called") },
        ( _, None) => { println!("No subcommand")},
        ( _, Some(_v)) => { println!("Bad subscommand")}
    };
}

pub fn add_nodes_to_subnet(subnet: String) {
    println!("{}", subnet);
    println!("Not implemented yet (add_nodes)");
}

async fn remove_dead_nodes_from_subnet(subnet: &str, url: &str, client: &Client, dryrun: types::DryRun) -> Result<(), Error> {
    println!("Not implemented yet (remove_nodes)");
    let nodes_to_remove = get_dead_nodes(&subnet, url, client).await?;
    let assumed_removed = nodes_to_remove.nodes.clone();
    let node_count = assumed_removed.len() as i32;
    let body = types::DecentralizedNodeQuery{
        subnet: subnet.to_string(),
        removals: Some(assumed_removed.clone()),
        node_count
    };
    let best_added_nodes = client.post(url).json(&body).send().await?.json::<types::BestNodesResponse>().await?.nodes.clone();
    println!("The current dead nodes are {:?}, and the nodes that we would like to add are {:?}", assumed_removed, best_added_nodes);
    match dryrun {
        types::DryRun::True => {
            add_and_remove_nodes(assumed_removed, best_added_nodes);
        },
        types::DryRun::False => {
            println!("Not running this command, feel free to double check, if you want to run for real set flag --iwanttodothisforrealipromiseiknowwhatimdoing")
        }
    }
    Ok(())
}

async fn get_decentralized_nodes(url: &str, client: Client, params: types::DecentralizedNodeQuery) -> Result<serde_json::Value, anyhow::Error> {
    let resp = client.post(url).json(&params).send().await?.json::<Value>().await?;
    Ok(resp)
}

async fn get_dead_nodes(subnet: &str, url: &str, client: &Client) -> Result<types::NodesToRemoveResponse, anyhow::Error> {
    let resp = client.get(url).query(&[("subnet", subnet)]).send().await?.json::<types::NodesToRemoveResponse>().await?;
    Ok(resp)
}

pub fn add_and_remove_nodes(removed: Vec<String>, added: Vec<String>) {

}
