use ic_management_types::Network;
use std::str::FromStr;
use url::Url;

pub fn target_network() -> Network {
    Network::from_str(&std::env::var("NETWORK").expect("Missing NETWORK environment variable"))
        .expect("Invalid network")
}

pub fn nns_url() -> String {
    std::env::var("NNS_URL").expect("NNS_URL environment variable not provided")
}

pub fn nns_nodes_urls() -> Vec<Url> {
    vec![Url::parse(&nns_url()).expect("Cannot parse NNS_URL environment variable as a valid URL")]
}
