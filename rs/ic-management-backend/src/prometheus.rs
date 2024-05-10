use ic_management_types::Network;
use prometheus_http_query::Client;

pub fn client(network: &Network) -> Client {
    Client::try_from(network.get_prometheus_endpoint().as_str()).unwrap()
}
