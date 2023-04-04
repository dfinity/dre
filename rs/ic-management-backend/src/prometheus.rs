use ic_management_types::Network;
use prometheus_http_query::Client;

pub fn client(network: &Network) -> Client {
    match network {
        Network::Mainnet | Network::Staging => Client::try_from("https://prometheus.mainnet.dfinity.network").unwrap(),
        _ => Client::try_from("https://prometheus.testnet.dfinity.network").unwrap(),
    }
}
