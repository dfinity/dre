use confy::load;
use serde::{Serialize, Deserialize};
use clap::{App, load_yaml};
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct OperatorConfig {
    hsm_pin: String,
    hsm_slot: String,
    hsm_key_id: String,
    neuron_id: String,
    proposal_url: String,
    ic_admin_cmd: String
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let args = App::from(yaml).get_matches();
    match args.value_of("subcommand") {
        Some(v) => {
            if v == "add-nodes" {
                return add_nodes_to_subnet(args.value_of("subcommand").unwrap().to_string());
            }
            else if v == "remove-dead-nodes" {
                return remove_dead_nodes_from_subnet(&args.value_of("subcommand").unwrap());
            }
            }
        }
    }
    println!("{:?}", yaml);
    let cfg: OperatorConfig = confy::load_path("management_config.toml").unwrap();
    println!("{:?}", cfg);
    println!("Hello, world!");
}

pub fn add_nodes_to_subnet(subnet: String) {
    println!("Not implemented yet");
}

pub fn remove_dead_nodes_from_subnet(subnet: &str) {
    println!("Not implemented yet");
}