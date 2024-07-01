use axum::extract::State;

use crate::handlers::Server;

mod criteria_tests;
mod rate_tests;

const RATE: u64 = 42;
const CRITERIA: &[&str] = &["test.*"];

fn criteria() -> Vec<String> {
    CRITERIA.iter().map(|s| s.to_string()).collect()
}

fn server() -> State<Server> {
    server_with_rate_and_criteria(RATE, criteria())
}

fn server_with_rate_and_criteria(rate: u64, criteria: Vec<String>) -> State<Server> {
    State(Server::new(slog::Logger::root(slog::Discard, slog::o!()), rate, criteria))
}

fn server_with_rate(rate: u64) -> State<Server> {
    server_with_rate_and_criteria(rate, vec![])
}

fn server_with_criteria(criteria: Vec<String>) -> State<Server> {
    server_with_rate_and_criteria(RATE, criteria)
}
