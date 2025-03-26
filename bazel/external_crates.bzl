load("@rules_rust//crate_universe:defs.bzl", "crate", "crates_repository", "splicing_config")

def external_crates_repository():
    crates_repository(
        name = "crate_index_dre",
        annotations = {
            "ic-icrc1-ledger": [crate.annotation(
                build_script_data = [
                    "@ic-icrc1-archive//file",
                ],
                build_script_env = {
                    "IC_ICRC1_ARCHIVE_WASM_PATH": "$(execpath @ic-icrc1-archive//file)",
                },
                compile_data = [
                    "@ic-icrc1-archive//file",
                ],
                rustc_env = {
                    "IC_ICRC1_ARCHIVE_WASM_PATH": "$(execpath @ic-icrc1-archive//file)",
                },
            )],
        },
        cargo_config = "//:.cargo/config.toml",
        cargo_lockfile = "//:Cargo.lock",
        isolated = False,
        lockfile = "//:Cargo.Bazel.lock",
        manifests = [
            "//:Cargo.toml",
            "//rs/canister-log-fetcher:Cargo.toml",
            "//rs/cli:Cargo.toml",
            "//rs/decentralization:Cargo.toml",
            "//rs/ic-canisters:Cargo.toml",
            "//rs/ic-management-backend:Cargo.toml",
            "//rs/ic-management-types:Cargo.toml",
            "//rs/ic-observability/config-writer-common:Cargo.toml",
            "//rs/ic-observability/log-noise-filter-backend:Cargo.toml",
            "//rs/ic-observability/log-noise-filter-downloader:Cargo.toml",
            "//rs/ic-observability/multiservice-discovery:Cargo.toml",
            "//rs/ic-observability/general-testnet-service-discovery:Cargo.toml",
            "//rs/ic-observability/multiservice-discovery-downloader:Cargo.toml",
            "//rs/ic-observability/multiservice-discovery-shared:Cargo.toml",
            "//rs/ic-observability/node-status-updater:Cargo.toml",
            "//rs/ic-observability/obs-canister-clients:Cargo.toml",
            "//rs/ic-observability/prometheus-config-updater:Cargo.toml",
            "//rs/ic-observability/service-discovery:Cargo.toml",
            "//rs/ic-observability/sns-downloader:Cargo.toml",
            "//rs/log-fetcher:Cargo.toml",
            "//rs/slack-notifications:Cargo.toml",
            "//rs/node-provider-rewards:Cargo.toml",
            "//rs/dre-canisters/node-provider-rewards:Cargo.toml",
            "//rs/dre-canisters/node-provider-rewards-lib:Cargo.toml",
            "//rs:dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics/Cargo.toml",
            "//rs/dre-canisters/trustworthy-node-metrics/src/trustworthy-node-metrics-types:Cargo.toml",
            "//rs:dre-canisters/node_status_canister/src/node_status_canister_backend/Cargo.toml"
        ],
        splicing_config = splicing_config(
            resolver_version = "2",
        ),
    )
