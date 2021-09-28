use confy::load_path;
use clap::{AppSettings, Clap};
mod types;
mod ops;
mod utils;
mod cli_types;
mod state_worker;
use cli_types::{SingleNode, MultipleNodes, Opts, SubCommand};
use std::sync::Arc;
use state_worker::ReplacementStateWorker;
use lazy_static;

lazy_static! {
static cfg: cli_types::OperatorConfig = load_path("management_config.toml").unwrap();
}
fn main() {
    let client = reqwest::Client::new();
    let opts = Opts::parse();
    let sqlite_file = "statemachine.sqlite";
    let sqlite_connection_manager = r2d2_sqlite::SqliteConnectionManager::file(sqlite_file);
    let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQLite connection pool");
    let pool_arc = Arc::new(sqlite_pool);
    let pool = pool_arc.clone();
    cfg = utils::merge_opts_into_cfg(&opts, cfg);
    let worker = ReplacementStateWorker::new(
        pool,
        &cfg
    );
    match &opts.subcommand {
        SubCommand::ReplaceSingleArbitrary(v) => { ops::add_and_remove_single_node(v.clone(), &opts, cfg, &worker) },
        _ => { println!("Not implemented yet")}
    }
}