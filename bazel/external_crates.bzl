load("@rules_rust//crate_universe:defs.bzl", "crate", "crates_repository", "splicing_config")

def external_crates_repository():
    crates_repository(
        name = "crate_index_release",
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
            "ic-adapter-metrics-service": [crate.annotation(
                build_script_data = [
                    "@com_google_protobuf//:protoc",
                    "@com_google_protobuf//:well_known_protos",
                ],
                build_script_env = {
                    "PROTOC": "$(execpath @com_google_protobuf//:protoc)",
                    "PROTOC_INCLUDE": "../com_google_protobuf/src",
                },
            )],
        },
        cargo_config = "//:bazel/cargo.config",
        cargo_lockfile = "//:Cargo.lock",
        isolated = True,
        lockfile = "//:Cargo.Bazel.lock",
        manifests = [
            "//:Cargo.toml",
            "//:rs/cli/Cargo.toml",
            "//:rs/decentralization/Cargo.toml",
            "//:rs/ic-management-backend/Cargo.toml",
            "//:rs/np-notifications/Cargo.toml",
            "//:rs/slack-notifications/Cargo.toml",
            "//rs/ic-management-types:Cargo.toml",
        ],
        splicing_config = splicing_config(
            resolver_version = "2",
        ),
    )
