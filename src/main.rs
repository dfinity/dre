use confy::load_path;
use clap::{AppSettings, Clap};
mod types;
mod ops;
mod utils;
mod cli_types;
mod state_worker;
use cli_types::{SingleNode, MultipleNodes, Opts, SubCommand};
use std::sync::Arc;
use std::borrow::Borrow;
use r2d2_sqlite;
use r2d2;
use rusqlite::params;
use state_worker::ReplacementStateWorker;

fn main() {
    let client = reqwest::Client::new();
    let opts = Opts::parse();
    let sqlite_file = "statemachine.sqlite";
    let sqlite_connection_manager = r2d2_sqlite::SqliteConnectionManager::file(sqlite_file);
    let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQLite connection pool");
    let pool_arc = Arc::new(sqlite_pool);
    let pool = pool_arc.clone();
    let worker = ReplacementStateWorker {

    };
    pool.get()
        .expect("Unable to get threadpool connection")
        .execute(
            "CREATE TABLE IF NOT EXISTS replacement_queue (waiting TEXT removed TEXT)", params![]
        )
        .expect("Unable to create needed database table");
    let mut cfg: cli_types::OperatorConfig = load_path("management_config.toml").unwrap();
    cfg = utils::merge_opts_into_cfg(&opts, cfg);
    match &opts.subcommand {
        SubCommand::ReplaceSingleArbitrary(v) => { ops::add_and_remove_single_node(v.clone(), &opts, cfg, worker },
        _ => { println!("Not implemented yet")}
    }
}