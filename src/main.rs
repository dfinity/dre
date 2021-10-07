#[macro_use]
extern crate diesel;
use clap::Clap;
use diesel::prelude::*;
use dotenv::dotenv;
use log::debug;

mod autoops_types;
mod cli_types;
mod models;
mod ops_subnet_node_replace;
mod schema;
mod utils;
use cli_types::{Opts, SubCommand};
use utils::env_cfg;

fn main() -> Result<(), anyhow::Error> {
    init_env();

    let subnet_update_nodes_state = init_sqlite_connect();
    let cli_opts = Opts::parse();
    cli_types::load_command_line_config_override(&cli_opts);

    // Start of actually doing stuff with commands.
    match &cli_opts.subcommand {
        SubCommand::SubnetUpdateNodes(nodes) => {
            match ops_subnet_node_replace::subnet_nodes_replace(&subnet_update_nodes_state, nodes) {
                Ok(stdout) => {
                    println!("{}", stdout);
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }
        _ => Err(anyhow::anyhow!("Not implemented yet")),
    }
}

fn init_sqlite_connect() -> SqliteConnection {
    debug!("Initializing the SQLite connection.");
    let database_url = env_cfg("DATABASE_URL");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn init_env() {
    if std::env::var("RUST_LOG").is_err() {
        // Set the default log level to info
        std::env::set_var("LOG_LEVEL", "info");
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");

    dotenv()
        .expect(".env file not found. Please copy env.template to .env and adjust configuration.");
}
