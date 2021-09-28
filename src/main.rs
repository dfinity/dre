use confy::load_path;
use clap::{AppSettings, Clap};
mod types;
mod ops;
mod utils;
mod cli_types;
mod state_worker;
use cli_types::{SingleNode, MultipleNodes, Opts, SubCommand};
use std::sync::Arc;
use std::sync::Once;
use state_worker::ReplacementStateWorker;
use threadpool::ThreadPool;
use lazy_static::lazy_static;

lazy_static! {
    static ref merged_opts: cli_types::OperatorConfig = {
        let init_file = load_path("management_config.toml").unwrap();
        let cli_opts = Opts::parse();
        utils::merge_opts_into_cfg(&cli_opts, &init_file)
    };

}

fn main() {
    let client = reqwest::Client::new();
    let load_opts: Opts = Opts::parse();
    let load_cfg: cli_types::OperatorConfig = load_path("management_config.toml").unwrap();
    // CFG needs a static lifetime for the state worker, this sync code ensures that initialization (merging) of the two structs is complete
    let sqlite_file = "statemachine.sqlite";
    let sqlite_connection_manager = r2d2_sqlite::SqliteConnectionManager::file(sqlite_file);
    let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQLite connection pool");
    let pool_arc = Arc::new(sqlite_pool);
    let pool = pool_arc.clone();
    let worker = Arc::new(ReplacementStateWorker::new(
        pool,
        &merged_opts
    ));
    let thread_clone = worker.clone();
    let worker_pool = ThreadPool::new(1);
    worker_pool.execute(move || thread_clone.update_loop());
    match &load_opts.subcommand {
        SubCommand::ReplaceSingleArbitrary(v) => { ops::add_and_remove_single_node(v.clone(), &merged_opts, &worker) },
        _ => { println!("Not implemented yet")}
    }
}