use ic_management_types::Network;
use prometheus_http_query::Client;

pub fn client(network: &Network) -> Client {
    match network {
        Network::Mainnet => Client::try_from("https://victoria.mainnet.dfinity.network/select/0/prometheus/").unwrap(),
        _ => Client::try_from("https://ic-metrics-victoria.ch1-obsstage1.dfinity.network/select/0/prometheus").unwrap(),
    }
}
