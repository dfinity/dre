use ic_management_types::Network;
use std::str::FromStr;
use url::Url;

pub fn target_network() -> Network {
    Network::from_str(&std::env::var("NETWORK").expect("Missing NETWORK environment variable"))
        .expect("Invalid network")
}

pub fn get_nns_url_string_from_target_network(target_network: &Network) -> String {
    match std::env::var("NNS_URL") {
        Ok(nns_url) => nns_url,
        Err(_) => match target_network {
            Network::Mainnet => "https://ic0.app".to_string(),
            Network::Staging => "http://[2600:3004:1200:1200:5000:11ff:fe37:c55d]:8080".to_string(),
            _ => panic!(
                "Cannot get NNS URL for target network {}. Please set NNS_URL environment variable",
                target_network
            ),
        },
    }
}

pub fn get_nns_url_vec_from_target_network(target_network: &Network) -> Vec<Url> {
    get_nns_url_string_from_target_network(target_network)
        .split(',')
        .map(|s| Url::parse(s).unwrap_or_else(|_| panic!("Cannot parse {} as a valid NNS URL", s)))
        .collect()
}
