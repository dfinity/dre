mod endpoints;
mod factsdb;
mod git_ic_repo;
mod gitlab_dfinity;
mod health;
mod prometheus;
mod proposal;
mod public_dashboard;
mod registry;
mod release;
mod subnets;

use dotenv::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let target_network = config::target_network();
    let listen_port = std::env::var("BACKEND_PORT")
        .map(|p| {
            p.parse()
                .expect("Unable to parse BACKEND_PORT environment variable as a valid port")
        })
        .unwrap_or(8080);
    endpoints::run_backend(target_network, "0.0.0.0", listen_port, false, None).await
}
