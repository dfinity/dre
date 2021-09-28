use confy::load_path;
use clap::{Clap};
mod types;
mod ops;
mod utils;
mod cli_types;
mod state_worker;
use cli_types::{Opts, SubCommand};
use std::sync::Arc;
use state_worker::ReplacementStateWorker;
use threadpool::ThreadPool;
use lazy_static::lazy_static;

lazy_static! {
    static ref CLI_OPTS: Opts = Opts::parse();
    static ref MERGED_OPTS: cli_types::OperatorConfig = {
        let init_file = load_path("management_config.toml").unwrap();
        utils::merge_opts_into_cfg(&CLI_OPTS, &init_file)
    };

}

fn main() {
    let client = reqwest::Client::new();

    // SQLite cconnection pool initialization
    let sqlite_file = "statemachine.sqlite";
    let sqlite_connection_manager = r2d2_sqlite::SqliteConnectionManager::file(sqlite_file);
    let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQLite connection pool");
    let pool_arc = Arc::new(sqlite_pool);
    let pool = pool_arc.clone();
    

    //State worker initialization
    let worker = Arc::new(ReplacementStateWorker::new(
        pool,
        &MERGED_OPTS
    ));
    let thread_clone = worker.clone();
    let worker_pool = ThreadPool::new(1);
    worker_pool.execute(move || thread_clone.update_loop());

    //Start of actually doing stuff with commands.
    match &CLI_OPTS.subcommand {
        SubCommand::ReplaceSingleArbitrary(v) => { ops::add_and_remove_single_node(v.clone(), &MERGED_OPTS, &worker) },
        _ => { println!("Not implemented yet")}
    }
}