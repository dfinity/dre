/// Build mainnet Wasm for Node Metrics Collector Canister
pub fn build_mainnet_metrics_collector_wasm() -> Wasm {
    let features = [];
    Project::cargo_bin_maybe_from_env("node-metrics-collector-canister", &features)
}
