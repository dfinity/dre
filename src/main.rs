use clap::Clap;
use confy::load_path;
mod cli_types;
mod ops;
mod states;
mod types;
mod utils;
use cli_types::{Opts, SubCommand};
use lazy_static::lazy_static;
use states::NodeReplacementStateDb;
use std::sync::Arc;
use threadpool::ThreadPool;

lazy_static! {
    static ref CLI_OPTS: Opts = Opts::parse();
    static ref MERGED_OPTS: cli_types::OperatorConfig = {
        let init_file = load_path("management_config.toml").unwrap();
        utils::merge_opts_into_cfg(&CLI_OPTS, &init_file)
    };
}

fn main() {
    let _client = reqwest::Client::new();

    let worker = sqlite_init_worker();

    // Start of actually doing stuff with commands.
    match &CLI_OPTS.subcommand {
        SubCommand::ReplaceNodeManual(v) => {
            ops::add_and_remove_single_node(v.clone(), &*MERGED_OPTS, &worker)
        }
        _ => {
            println!("Not implemented yet")
        }
    }
}

fn sqlite_init_worker() -> Arc<NodeReplacementStateDb> {
    // Initialize SQLite connection pool
    let sqlite_file_name = "state.sqlite";
    let sqlite_connection_manager = r2d2_sqlite::SqliteConnectionManager::file(sqlite_file_name);
    let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
        .expect("Failed to create r2d2 SQLite connection pool");
    let pool_arc = Arc::new(sqlite_pool);
    let pool = pool_arc;

    // State worker initialization
    let worker = Arc::new(NodeReplacementStateDb::new(pool, MERGED_OPTS.clone()));
    let thread_clone = worker.clone();
    let worker_pool = ThreadPool::new(1);
    worker_pool.execute(move || thread_clone.update_loop());
    worker
}
