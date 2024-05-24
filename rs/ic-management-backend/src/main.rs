mod endpoints;
mod git_ic_repo;
mod health;
mod node_labels;
mod prometheus;
mod proposal;
mod public_dashboard;
mod registry;
mod release;
mod subnets;

use clap::Parser;
use dotenv::dotenv;
use url::Url;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let args = Cli::parse();
    let target_network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
        .await
        .expect("Failed to create network");

    let listen_port = std::env::var("BACKEND_PORT")
        .map(|p| p.parse().expect("Unable to parse BACKEND_PORT environment variable as a valid port"))
        .unwrap_or(8080);
    endpoints::run_backend(&target_network, "0.0.0.0", listen_port, false, None).await
}

#[derive(Parser, Debug)]
#[clap(about, version)]
struct Cli {
    // Target network. Can be one of: "mainnet", "staging", or an arbitrary "<testnet>" name
    #[clap(long, env = "NETWORK", default_value = "mainnet")]
    network: String,

    // NNS_URLs for the target network, comma separated.
    // The argument is mandatory for testnets, and is optional for mainnet and staging
    #[clap(long, env = "NNS_URLS", aliases = &["registry-url", "nns-url"], value_delimiter = ',')]
    pub nns_urls: Vec<Url>,
}
