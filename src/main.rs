#[macro_use]
extern crate diesel;
use clap::Parser;
use diesel::prelude::*;
use dotenv::dotenv;
use log::{debug, info};
use utils::env_cfg;
mod autoops_types;
mod cli;
mod ic_admin;
mod model_proposals;
mod model_subnet_update_nodes;
mod ops_subnet_node_replace;
mod schema;
mod utils;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_env();

    let db_connection = init_sqlite_connect();
    let cli_opts = cli::Opts::parse();
    init_logger();

    // Start of actually doing stuff with commands.
    match &cli_opts.subcommand {
        cli::Commands::SubnetReplaceNodes { subnet, add, remove } => {
            let ica = ic_admin::Cli::from(&cli_opts);
            match ops_subnet_node_replace::subnet_nodes_replace(
                &ica,
                &db_connection,
                subnet,
                add.clone(),
                remove.clone(),
            ) {
                Ok(stdout) => {
                    println!("{}", stdout);
                    Ok(())
                }
                Err(err) => Err(err),
            }?;
            loop {
                let pending = ops_subnet_node_replace::check_and_submit_proposals_subnet_add_nodes(
                    &ica,
                    &db_connection,
                    &subnet.to_string(),
                )?;
                if pending {
                    info!("There are pending proposals. Waiting 10 seconds");
                    std::thread::sleep(std::time::Duration::from_secs(10));
                } else {
                    break;
                }
            }
            info!("There are no more pending proposals. Exiting...");
            Ok(())
        }

        cli::Commands::DerToPrincipal { path } => {
            let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(path)?);
            println!("{}", principal);
            Ok(())
        }
    }
}

fn init_sqlite_connect() -> SqliteConnection {
    debug!("Initializing the SQLite connection.");
    let home_path = std::env::var("HOME").expect("Getting HOME environment variable failed.");
    let database_url = env_cfg("DATABASE_URL").replace("~/", format!("{}/", home_path).as_str());
    let database_url_dirname = std::path::Path::new(&database_url)
        .parent()
        .expect("Getting the dirname for the database_url failed.");
    std::fs::create_dir_all(database_url_dirname).expect("Creating the directory for the database file failed.");
    SqliteConnection::establish(&database_url).unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn init_env() {
    dotenv().expect(".env file not found. Please copy env.template to .env and adjust configuration.");
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Set a default logging level: info, if nothing else specified in environment
                // variables RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
