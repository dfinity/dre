use ic_management_types::Network;
use prometheus_http_query::Client;

pub fn client(network: &Network) -> Client {
    match network {
        Network::Mainnet | Network::Staging => {
            Client::try_from("https://vmselect.ch1-obs1.dfinity.network/select/0/prometheus").unwrap()
        }
        _ => Client::try_from("https://prometheus.testnet.dfinity.network").unwrap(),
    }
}
