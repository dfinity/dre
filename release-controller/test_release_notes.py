from release_notes import prepare_release_notes


def test_release_notes(mocker):
    guest_os_packages_all = [
        "bazel",
        "cpp",
        "gitlab-ci/src/artifacts",
        "ic-os/bootloader",
        "ic-os/components",
        "ic-os/guestos",
        "ic-os/guestos/context",
        "ic-os/guestos/envs/prod",
        "ic-os/hostos",
        "ic-os/hostos/context",
        "ic-os/hostos/envs/prod",
        "ic-os/setupos",
        "ic-os/setupos/context",
        "ic-os/setupos/envs/prod",
        "packages/ic-ledger-hash-of",
        "packages/icrc-ledger-client",
        "packages/icrc-ledger-types",
        "publish/binaries",
        "rs/artifact_pool",
        "rs/async_utils",
        "rs/bitcoin/adapter",
        "rs/bitcoin/client",
        "rs/bitcoin/consensus",
        "rs/bitcoin/replica_types",
        "rs/bitcoin/service",
        "rs/boundary_node/ic_boundary",
        "rs/canister_client",
        "rs/canister_client/sender",
        "rs/canister_sandbox",
        "rs/canonical_state",
        "rs/canonical_state/certification_version",
        "rs/canonical_state/tree_hash",
        "rs/certification",
        "rs/certification/test-utils",
        "rs/config",
        "rs/consensus",
        "rs/consensus/utils",
        "rs/constants",
        "rs/crypto",
        "rs/crypto/ecdsa_secp256k1",
        "rs/crypto/ecdsa_secp256r1",
        "rs/crypto/ed25519",
        "rs/crypto/for_verification_only",
        "rs/crypto/getrandom_for_wasm",
        "rs/crypto/iccsa",
        "rs/crypto/interfaces/sig_verification",
        "rs/crypto/internal/crypto_lib/basic_sig/cose",
        "rs/crypto/internal/crypto_lib/basic_sig/der_utils",
        "rs/crypto/internal/crypto_lib/basic_sig/ecdsa_secp256k1",
        "rs/crypto/internal/crypto_lib/basic_sig/ecdsa_secp256r1",
        "rs/crypto/internal/crypto_lib/basic_sig/ed25519",
        "rs/crypto/internal/crypto_lib/basic_sig/iccsa",
        "rs/crypto/internal/crypto_lib/basic_sig/rsa_pkcs1",
        "rs/crypto/internal/crypto_lib/bls12_381/type",
        "rs/crypto/internal/crypto_lib/hmac",
        "rs/crypto/internal/crypto_lib/multi_sig/bls12_381",
        "rs/crypto/internal/crypto_lib/seed",
        "rs/crypto/internal/crypto_lib/sha2",
        "rs/crypto/internal/crypto_lib/threshold_sig/bls12_381",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa/fe-derive",
        "rs/crypto/internal/crypto_lib/tls",
        "rs/crypto/internal/crypto_lib/types",
        "rs/crypto/internal/crypto_service_provider",
        "rs/crypto/internal/logmon",
        "rs/crypto/internal/test_vectors",
        "rs/crypto/node_key_generation",
        "rs/crypto/node_key_validation",
        "rs/crypto/node_key_validation/tls_cert_validation",
        "rs/crypto/prng",
        "rs/crypto/secrets_containers",
        "rs/crypto/sha2",
        "rs/crypto/standalone-sig-verifier",
        "rs/crypto/temp_crypto",
        "rs/crypto/temp_crypto/temp_vault",
        "rs/crypto/test_utils/ni-dkg",
        "rs/crypto/tls_interfaces",
        "rs/crypto/tree_hash",
        "rs/crypto/utils/basic_sig",
        "rs/crypto/utils/canister_threshold_sig",
        "rs/crypto/utils/ni_dkg",
        "rs/crypto/utils/threshold_sig",
        "rs/crypto/utils/threshold_sig_der",
        "rs/crypto/utils/tls",
        "rs/cup_explorer",
        "rs/cycles_account_manager",
        "rs/embedders",
        "rs/execution_environment",
        "rs/http_endpoints/metrics",
        "rs/http_endpoints/public",
        "rs/http_utils",
        "rs/https_outcalls/adapter",
        "rs/https_outcalls/client",
        "rs/https_outcalls/consensus",
        "rs/https_outcalls/service",
        "rs/ic_os/config",
        "rs/ic_os/dflate",
        "rs/ic_os/diroid",
        "rs/ic_os/fstrim_tool",
        "rs/ic_os/guestos_tool",
        "rs/ic_os/hostos_tool",
        "rs/ic_os/inject_files",
        "rs/ic_os/network",
        "rs/ic_os/nft_exporter",
        "rs/ic_os/nss_icos",
        "rs/ic_os/partition_tools",
        "rs/ic_os/setupos_tool",
        "rs/ic_os/utils",
        "rs/ic_os/vsock/guest",
        "rs/ic_os/vsock/host",
        "rs/ic_os/vsock/vsock_lib",
        "rs/ingress_manager",
        "rs/interfaces",
        "rs/interfaces/adapter_client",
        "rs/interfaces/certified_stream_store",
        "rs/interfaces/registry",
        "rs/interfaces/state_manager",
        "rs/memory_tracker",
        "rs/messaging",
        "rs/monitoring/adapter_metrics/client",
        "rs/monitoring/adapter_metrics/server",
        "rs/monitoring/adapter_metrics/service",
        "rs/monitoring/backtrace",
        "rs/monitoring/logger",
        "rs/monitoring/metrics",
        "rs/monitoring/pprof",
        "rs/monitoring/tracing",
        "rs/nervous_system",
        "rs/nervous_system/clients",
        "rs/nervous_system/collections/union_multi_map",
        "rs/nervous_system/common",
        "rs/nervous_system/common/build_metadata",
        "rs/nervous_system/governance",
        "rs/nervous_system/lock",
        "rs/nervous_system/neurons_fund",
        "rs/nervous_system/proto",
        "rs/nervous_system/proxied_canister_calls_tracker",
        "rs/nervous_system/root",
        "rs/nervous_system/runtime",
        "rs/nervous_system/string",
        "rs/nns/cmc",
        "rs/nns/common",
        "rs/nns/constants",
        "rs/nns/governance/api",
        "rs/orchestrator",
        "rs/orchestrator/dashboard",
        "rs/orchestrator/image_upgrader",
        "rs/orchestrator/registry_replicator",
        "rs/p2p/artifact_manager",
        "rs/p2p/consensus_manager",
        "rs/p2p/peer_manager",
        "rs/p2p/quic_transport",
        "rs/p2p/state_sync_manager",
        "rs/phantom_newtype",
        "rs/protobuf",
        "rs/query_stats",
        "rs/recovery",
        "rs/registry/canister",
        "rs/registry/client",
        "rs/registry/fake",
        "rs/registry/helpers",
        "rs/registry/keys",
        "rs/registry/local_store",
        "rs/registry/nns_data_provider",
        "rs/registry/nns_data_provider_wrappers",
        "rs/registry/proto",
        "rs/registry/proto_data_provider",
        "rs/registry/provisional_whitelist",
        "rs/registry/regedit",
        "rs/registry/routing_table",
        "rs/registry/subnet_features",
        "rs/registry/subnet_type",
        "rs/registry/transport",
        "rs/replay",
        "rs/replica",
        "rs/replica/setup_ic_network",
        "rs/replicated_state",
        "rs/rosetta-api/icp_ledger",
        "rs/rosetta-api/icrc1",
        "rs/rosetta-api/icrc1/archive",
        "rs/rosetta-api/icrc1/ledger",
        "rs/rosetta-api/icrc1/tokens_u64",
        "rs/rosetta-api/ledger_canister_core",
        "rs/rosetta-api/ledger_core",
        "rs/rust_canisters/canister_log",
        "rs/rust_canisters/canister_profiler",
        "rs/rust_canisters/dfn_candid",
        "rs/rust_canisters/dfn_core",
        "rs/rust_canisters/dfn_http",
        "rs/rust_canisters/dfn_http_metrics",
        "rs/rust_canisters/dfn_protobuf",
        "rs/rust_canisters/http_types",
        "rs/rust_canisters/on_wire",
        "rs/sns/governance",
        "rs/sns/governance/proposal_criticality",
        "rs/sns/governance/proposals_amount_total_limit",
        "rs/sns/governance/token_valuation",
        "rs/sns/root",
        "rs/sns/swap",
        "rs/sns/swap/proto_library",
        "rs/state_layout",
        "rs/state_manager",
        "rs/state_tool",
        "rs/sys",
        "rs/system_api",
        "rs/test_utilities/io",
        "rs/test_utilities/metrics",
        "rs/tree_deserializer",
        "rs/types/base_types",
        "rs/types/error_types",
        "rs/types/management_canister_types",
        "rs/types/types",
        "rs/types/wasm_types",
        "rs/utils",
        "rs/utils/lru_cache",
        "rs/utils/thread",
        "rs/validator",
        "rs/wasm_transform",
        "rs/xnet/endpoint",
        "rs/xnet/hyper",
        "rs/xnet/payload_builder",
        "rs/xnet/uri",
        "toolchains/sysimage",
    ]

    bazel_packages_all = [
        "bazel",
        "bazel/candid_integration_tests",
        "bazel/exporter",
        "bazel/inject_version_into_wasm_tests",
        "bazel/proto",
        "bazel/sanitizers_enabled_env",
        "bazel/tlaplus",
        "bazel/tools/cmpbuildlogs",
        "cpp",
        "gitlab-ci/config",
        "gitlab-ci/src/artifacts",
        "gitlab-ci/src/git_changes/test_data/change_file_ignore/after/rs",
        "gitlab-ci/src/git_changes/test_data/change_file_ignore/before/rs",
        "hs/spec_compliance",
        "ic-os",
        "ic-os/bootloader",
        "ic-os/boundary-guestos",
        "ic-os/boundary-guestos/context",
        "ic-os/boundary-guestos/envs/dev",
        "ic-os/boundary-guestos/envs/prod",
        "ic-os/components",
        "ic-os/dev-tools/bare_metal_deployment",
        "ic-os/guestos",
        "ic-os/guestos/context",
        "ic-os/guestos/envs/dev",
        "ic-os/guestos/envs/dev-malicious",
        "ic-os/guestos/envs/local-base-dev",
        "ic-os/guestos/envs/local-base-prod",
        "ic-os/guestos/envs/prod",
        "ic-os/hostos",
        "ic-os/hostos/context",
        "ic-os/hostos/envs/dev",
        "ic-os/hostos/envs/local-base-dev",
        "ic-os/hostos/envs/local-base-prod",
        "ic-os/hostos/envs/prod",
        "ic-os/setupos",
        "ic-os/setupos/context",
        "ic-os/setupos/envs/dev",
        "ic-os/setupos/envs/local-base-dev",
        "ic-os/setupos/envs/local-base-prod",
        "ic-os/setupos/envs/prod",
        "packages/ic-ledger-hash-of",
        "packages/ic-signature-verification",
        "packages/ic-vetkd-utils",
        "packages/icrc-ledger-agent",
        "packages/icrc-ledger-client",
        "packages/icrc-ledger-client-cdk",
        "packages/icrc-ledger-types",
        "packages/pocket-ic",
        "pre-commit",
        "publish",
        "publish/binaries",
        "publish/canisters",
        "publish/malicious",
        "publish/tests",
        "rs",
        "rs/artifact_pool",
        "rs/async_utils",
        "rs/backup",
        "rs/bitcoin/adapter",
        "rs/bitcoin/adapter/test_utils",
        "rs/bitcoin/ckbtc/agent",
        "rs/bitcoin/ckbtc/kyt",
        "rs/bitcoin/ckbtc/minter",
        "rs/bitcoin/ckbtc/spec",
        "rs/bitcoin/client",
        "rs/bitcoin/consensus",
        "rs/bitcoin/mock",
        "rs/bitcoin/replica_types",
        "rs/bitcoin/service",
        "rs/boundary_node/canary_proxy",
        "rs/boundary_node/certificate_issuance/certificate_issuer",
        "rs/boundary_node/certificate_issuance/certificate_orchestrator",
        "rs/boundary_node/certificate_issuance/certificate_orchestrator_interface",
        "rs/boundary_node/certificate_issuance/certificate_syncer",
        "rs/boundary_node/denylist_updater",
        "rs/boundary_node/discower_bowndary",
        "rs/boundary_node/ic_balance_exporter",
        "rs/boundary_node/ic_boundary",
        "rs/boundary_node/icx_proxy",
        "rs/boundary_node/prober",
        "rs/boundary_node/systemd_journal_gatewayd_shim",
        "rs/canister_client",
        "rs/canister_client/sender",
        "rs/canister_sandbox",
        "rs/canonical_state",
        "rs/canonical_state/certification_version",
        "rs/canonical_state/tree_hash",
        "rs/canonical_state/tree_hash/test_utils",
        "rs/certification",
        "rs/certification/test-utils",
        "rs/config",
        "rs/consensus",
        "rs/consensus/mocks",
        "rs/consensus/utils",
        "rs/constants",
        "rs/criterion_time",
        "rs/cross-chain",
        "rs/cross-chain/proposal-cli",
        "rs/crypto",
        "rs/crypto/ecdsa_secp256k1",
        "rs/crypto/ecdsa_secp256r1",
        "rs/crypto/ed25519",
        "rs/crypto/extended_bip32",
        "rs/crypto/for_verification_only",
        "rs/crypto/getrandom_for_wasm",
        "rs/crypto/iccsa",
        "rs/crypto/interfaces/sig_verification",
        "rs/crypto/internal/crypto_lib/basic_sig/cose",
        "rs/crypto/internal/crypto_lib/basic_sig/der_utils",
        "rs/crypto/internal/crypto_lib/basic_sig/ecdsa_secp256k1",
        "rs/crypto/internal/crypto_lib/basic_sig/ecdsa_secp256r1",
        "rs/crypto/internal/crypto_lib/basic_sig/ed25519",
        "rs/crypto/internal/crypto_lib/basic_sig/iccsa",
        "rs/crypto/internal/crypto_lib/basic_sig/iccsa/test_utils",
        "rs/crypto/internal/crypto_lib/basic_sig/rsa_pkcs1",
        "rs/crypto/internal/crypto_lib/bls12_381/type",
        "rs/crypto/internal/crypto_lib/bls12_381/vetkd",
        "rs/crypto/internal/crypto_lib/hmac",
        "rs/crypto/internal/crypto_lib/multi_sig/bls12_381",
        "rs/crypto/internal/crypto_lib/seed",
        "rs/crypto/internal/crypto_lib/sha2",
        "rs/crypto/internal/crypto_lib/threshold_sig/bls12_381",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa/fe-derive",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa/fuzz",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa/fuzz/cbor_deserialize_dealing_seed_corpus",
        "rs/crypto/internal/crypto_lib/threshold_sig/tecdsa/test_utils",
        "rs/crypto/internal/crypto_lib/tls",
        "rs/crypto/internal/crypto_lib/types",
        "rs/crypto/internal/crypto_service_provider",
        "rs/crypto/internal/crypto_service_provider/csp_proptest_utils",
        "rs/crypto/internal/crypto_service_provider/protobuf_generator",
        "rs/crypto/internal/csp_test_utils",
        "rs/crypto/internal/logmon",
        "rs/crypto/internal/test_vectors",
        "rs/crypto/node_key_generation",
        "rs/crypto/node_key_validation",
        "rs/crypto/node_key_validation/tls_cert_validation",
        "rs/crypto/prng",
        "rs/crypto/secrets_containers",
        "rs/crypto/sha2",
        "rs/crypto/sha3",
        "rs/crypto/standalone-sig-verifier",
        "rs/crypto/temp_crypto",
        "rs/crypto/temp_crypto/temp_vault",
        "rs/crypto/test_utils",
        "rs/crypto/test_utils/canister_sigs",
        "rs/crypto/test_utils/canister_threshold_sigs",
        "rs/crypto/test_utils/csp",
        "rs/crypto/test_utils/keygen",
        "rs/crypto/test_utils/keys",
        "rs/crypto/test_utils/local_csp_vault",
        "rs/crypto/test_utils/metrics",
        "rs/crypto/test_utils/multi_sigs",
        "rs/crypto/test_utils/ni-dkg",
        "rs/crypto/test_utils/reproducible_rng",
        "rs/crypto/test_utils/root_of_trust",
        "rs/crypto/test_utils/tls",
        "rs/crypto/tls_interfaces",
        "rs/crypto/tls_interfaces/mocks",
        "rs/crypto/tree_hash",
        "rs/crypto/tree_hash/fuzz",
        "rs/crypto/tree_hash/fuzz/check_witness_equality_utils",
        "rs/crypto/tree_hash/test_utils",
        "rs/crypto/utils/basic_sig",
        "rs/crypto/utils/canister_threshold_sig",
        "rs/crypto/utils/ni_dkg",
        "rs/crypto/utils/threshold_sig",
        "rs/crypto/utils/threshold_sig_der",
        "rs/crypto/utils/tls",
        "rs/cup_explorer",
        "rs/cycles_account_manager",
        "rs/depcheck",
        "rs/determinism_test",
        "rs/drun",
        "rs/embedders",
        "rs/embedders/fuzz",
        "rs/ethereum/cketh/minter",
        "rs/ethereum/cketh/test_utils",
        "rs/ethereum/evm-rpc-client",
        "rs/ethereum/ledger-suite-orchestrator",
        "rs/ethereum/ledger-suite-orchestrator/test_utils",
        "rs/ethereum/types",
        "rs/execution_environment",
        "rs/execution_environment/benches/lib",
        "rs/execution_environment/benches/management_canister/test_canister",
        "rs/execution_environment/fuzz",
        "rs/execution_environment/tools",
        "rs/fuzzers/bitcoin",
        "rs/fuzzers/candid",
        "rs/fuzzers/stable_structures",
        "rs/http_endpoints/fuzz",
        "rs/http_endpoints/metrics",
        "rs/http_endpoints/public",
        "rs/http_utils",
        "rs/https_outcalls/adapter",
        "rs/https_outcalls/client",
        "rs/https_outcalls/consensus",
        "rs/https_outcalls/service",
        "rs/ic_os/config",
        "rs/ic_os/deterministic_ips",
        "rs/ic_os/dflate",
        "rs/ic_os/diroid",
        "rs/ic_os/fstrim_tool",
        "rs/ic_os/guestos_tool",
        "rs/ic_os/hostos_tool",
        "rs/ic_os/inject_files",
        "rs/ic_os/launch-single-vm",
        "rs/ic_os/network",
        "rs/ic_os/nft_exporter",
        "rs/ic_os/nss_icos",
        "rs/ic_os/partition_tools",
        "rs/ic_os/release",
        "rs/ic_os/setupos-disable-checks",
        "rs/ic_os/setupos-inject-configuration",
        "rs/ic_os/setupos_tool",
        "rs/ic_os/utils",
        "rs/ic_os/vsock/guest",
        "rs/ic_os/vsock/host",
        "rs/ic_os/vsock/vsock_lib",
        "rs/ingress_manager",
        "rs/interfaces",
        "rs/interfaces/adapter_client",
        "rs/interfaces/certified_stream_store",
        "rs/interfaces/certified_stream_store/mocks",
        "rs/interfaces/mocks",
        "rs/interfaces/registry",
        "rs/interfaces/registry/mocks",
        "rs/interfaces/state_manager",
        "rs/interfaces/state_manager/mocks",
        "rs/memory_tracker",
        "rs/messaging",
        "rs/monitoring/adapter_metrics/client",
        "rs/monitoring/adapter_metrics/server",
        "rs/monitoring/adapter_metrics/service",
        "rs/monitoring/backtrace",
        "rs/monitoring/logger",
        "rs/monitoring/metrics",
        "rs/monitoring/pprof",
        "rs/monitoring/tracing",
        "rs/nervous_system",
        "rs/nervous_system/clients",
        "rs/nervous_system/collections/union_multi_map",
        "rs/nervous_system/common",
        "rs/nervous_system/common/build_metadata",
        "rs/nervous_system/common/test_canister",
        "rs/nervous_system/common/test_keys",
        "rs/nervous_system/common/test_utils",
        "rs/nervous_system/governance",
        "rs/nervous_system/humanize",
        "rs/nervous_system/integration_tests",
        "rs/nervous_system/lock",
        "rs/nervous_system/neurons_fund",
        "rs/nervous_system/neurons_fund/nfplot",
        "rs/nervous_system/proto",
        "rs/nervous_system/proto/protobuf_generator",
        "rs/nervous_system/proxied_canister_calls_tracker",
        "rs/nervous_system/root",
        "rs/nervous_system/runtime",
        "rs/nervous_system/string",
        "rs/nervous_system/tools/sync-with-released-nevous-system-wasms",
        "rs/nns/cmc",
        "rs/nns/common",
        "rs/nns/common/protobuf_generator",
        "rs/nns/constants",
        "rs/nns/governance",
        "rs/nns/governance/api",
        "rs/nns/governance/init",
        "rs/nns/governance/protobuf_generator",
        "rs/nns/gtc",
        "rs/nns/gtc/protobuf_generator",
        "rs/nns/gtc_accounts",
        "rs/nns/handlers/lifeline/impl",
        "rs/nns/handlers/lifeline/interface",
        "rs/nns/handlers/root/impl",
        "rs/nns/handlers/root/impl/protobuf_generator",
        "rs/nns/handlers/root/interface",
        "rs/nns/identity",
        "rs/nns/init",
        "rs/nns/inspector",
        "rs/nns/integration_tests",
        "rs/nns/nns-ui",
        "rs/nns/sns-wasm",
        "rs/nns/sns-wasm/protobuf_generator",
        "rs/nns/test_utils",
        "rs/nns/test_utils/golden_nns_state",
        "rs/nns/test_utils_macros",
        "rs/orchestrator",
        "rs/orchestrator/dashboard",
        "rs/orchestrator/image_upgrader",
        "rs/orchestrator/registry_replicator",
        "rs/p2p/artifact_downloader",
        "rs/p2p/artifact_manager",
        "rs/p2p/consensus_manager",
        "rs/p2p/memory_transport",
        "rs/p2p/peer_manager",
        "rs/p2p/quic_transport",
        "rs/p2p/state_sync_manager",
        "rs/p2p/test_utils",
        "rs/phantom_newtype",
        "rs/pocket_ic_server",
        "rs/prep",
        "rs/protobuf",
        "rs/protobuf/generator",
        "rs/query_stats",
        "rs/recovery",
        "rs/recovery/subnet_splitting",
        "rs/registry/admin",
        "rs/registry/admin-derive",
        "rs/registry/canister",
        "rs/registry/canister/protobuf_generator",
        "rs/registry/client",
        "rs/registry/fake",
        "rs/registry/helpers",
        "rs/registry/keys",
        "rs/registry/local_registry",
        "rs/registry/local_store",
        "rs/registry/local_store/artifacts",
        "rs/registry/nns_data_provider",
        "rs/registry/nns_data_provider_wrappers",
        "rs/registry/proto",
        "rs/registry/proto/generator",
        "rs/registry/proto_data_provider",
        "rs/registry/provisional_whitelist",
        "rs/registry/regedit",
        "rs/registry/routing_table",
        "rs/registry/subnet_features",
        "rs/registry/subnet_type",
        "rs/registry/transport",
        "rs/registry/transport/protobuf_generator",
        "rs/replay",
        "rs/replica",
        "rs/replica/setup_ic_network",
        "rs/replica_tests",
        "rs/replicated_state",
        "rs/replicated_state/fuzz",
        "rs/rosetta-api",
        "rs/rosetta-api/hardware_wallet_tests",
        "rs/rosetta-api/icp_ledger",
        "rs/rosetta-api/icp_ledger/archive",
        "rs/rosetta-api/icp_ledger/index",
        "rs/rosetta-api/icp_ledger/ledger",
        "rs/rosetta-api/icp_ledger/protobuf_generator",
        "rs/rosetta-api/icp_ledger/rosetta-integration-tests",
        "rs/rosetta-api/icp_ledger/test_utils",
        "rs/rosetta-api/icrc1",
        "rs/rosetta-api/icrc1/archive",
        "rs/rosetta-api/icrc1/benchmark/generator",
        "rs/rosetta-api/icrc1/benchmark/worker",
        "rs/rosetta-api/icrc1/index",
        "rs/rosetta-api/icrc1/index-ng",
        "rs/rosetta-api/icrc1/ledger",
        "rs/rosetta-api/icrc1/ledger/sm-tests",
        "rs/rosetta-api/icrc1/rosetta",
        "rs/rosetta-api/icrc1/rosetta/client",
        "rs/rosetta-api/icrc1/rosetta/runner",
        "rs/rosetta-api/icrc1/test_utils",
        "rs/rosetta-api/icrc1/tokens_u256",
        "rs/rosetta-api/icrc1/tokens_u64",
        "rs/rosetta-api/ledger_canister_blocks_synchronizer",
        "rs/rosetta-api/ledger_canister_blocks_synchronizer/test_utils",
        "rs/rosetta-api/ledger_canister_core",
        "rs/rosetta-api/ledger_core",
        "rs/rosetta-api/rosetta_core",
        "rs/rosetta-api/test_utils",
        "rs/rosetta-api/test_utils/sender_canister",
        "rs/rosetta-api/tvl",
        "rs/rosetta-api/tvl/xrc_mock",
        "rs/rust_canisters/call_tree_test",
        "rs/rust_canisters/canister_creator",
        "rs/rust_canisters/canister_log",
        "rs/rust_canisters/canister_profiler",
        "rs/rust_canisters/canister_serve",
        "rs/rust_canisters/canister_test",
        "rs/rust_canisters/dfn_candid",
        "rs/rust_canisters/dfn_core",
        "rs/rust_canisters/dfn_http",
        "rs/rust_canisters/dfn_http_metrics",
        "rs/rust_canisters/dfn_json",
        "rs/rust_canisters/dfn_macro",
        "rs/rust_canisters/dfn_protobuf",
        "rs/rust_canisters/downstream_calls_test",
        "rs/rust_canisters/ecdsa",
        "rs/rust_canisters/http_types",
        "rs/rust_canisters/memory_test",
        "rs/rust_canisters/on_wire",
        "rs/rust_canisters/pmap",
        "rs/rust_canisters/proxy_canister",
        "rs/rust_canisters/response_payload_test",
        "rs/rust_canisters/stable_memory_integrity",
        "rs/rust_canisters/stable_reader",
        "rs/rust_canisters/stable_structures",
        "rs/rust_canisters/statesync_test",
        "rs/rust_canisters/tests",
        "rs/rust_canisters/xnet_test",
        "rs/scenario_tests",
        "rs/sns/audit",
        "rs/sns/cli",
        "rs/sns/governance",
        "rs/sns/governance/proposal_criticality",
        "rs/sns/governance/proposals_amount_total_limit",
        "rs/sns/governance/protobuf_generator",
        "rs/sns/governance/token_valuation",
        "rs/sns/init",
        "rs/sns/init/protobuf_generator",
        "rs/sns/integration_tests",
        "rs/sns/root",
        "rs/sns/root/protobuf_generator",
        "rs/sns/swap",
        "rs/sns/swap/proto_library",
        "rs/sns/swap/protobuf_generator",
        "rs/sns/test_utils",
        "rs/starter",
        "rs/state_layout",
        "rs/state_machine_tests",
        "rs/state_manager",
        "rs/state_manager/spec",
        "rs/state_tool",
        "rs/sys",
        "rs/system_api",
        "rs/test_utilities",
        "rs/test_utilities/artifact_pool",
        "rs/test_utilities/compare_dirs",
        "rs/test_utilities/consensus",
        "rs/test_utilities/embedders",
        "rs/test_utilities/execution_environment",
        "rs/test_utilities/identity",
        "rs/test_utilities/in_memory_logger",
        "rs/test_utilities/io",
        "rs/test_utilities/load_wasm",
        "rs/test_utilities/logger",
        "rs/test_utilities/metrics",
        "rs/test_utilities/registry",
        "rs/test_utilities/serialization",
        "rs/test_utilities/state",
        "rs/test_utilities/time",
        "rs/test_utilities/tmpdir",
        "rs/test_utilities/types",
        "rs/tests",
        "rs/tests/boundary_nodes",
        "rs/tests/consensus",
        "rs/tests/consensus/orchestrator",
        "rs/tests/consensus/tecdsa",
        "rs/tests/consensus/tecdsa/utils",
        "rs/tests/consensus/utils",
        "rs/tests/cross_chain",
        "rs/tests/crypto",
        "rs/tests/driver",
        "rs/tests/execution",
        "rs/tests/financial_integrations",
        "rs/tests/financial_integrations/ckbtc",
        "rs/tests/financial_integrations/rosetta",
        "rs/tests/gix",
        "rs/tests/httpbin-rs",
        "rs/tests/ict",
        "rs/tests/ict/cmd",
        "rs/tests/message_routing",
        "rs/tests/message_routing/xnet",
        "rs/tests/nested",
        "rs/tests/networking",
        "rs/tests/networking/canister_http",
        "rs/tests/nns",
        "rs/tests/nns/ic_mainnet_nns_recovery",
        "rs/tests/nns/sns",
        "rs/tests/node",
        "rs/tests/query_stats",
        "rs/tests/query_stats/lib",
        "rs/tests/sdk",
        "rs/tests/test_canisters/http_counter",
        "rs/tests/test_canisters/kv_store",
        "rs/tests/test_canisters/message",
        "rs/tests/testing_verification",
        "rs/tests/testing_verification/spec_compliance",
        "rs/tests/testing_verification/testnets",
        "rs/tests/testing_verification/wabt-tests",
        "rs/tools/check_did",
        "rs/tree_deserializer",
        "rs/types/base_types",
        "rs/types/base_types/protobuf_generator",
        "rs/types/error_types",
        "rs/types/exhaustive_derive",
        "rs/types/management_canister_types",
        "rs/types/management_canister_types/fuzz",
        "rs/types/management_canister_types/fuzz/candid_seed_corpus_generation",
        "rs/types/types",
        "rs/types/types_test_utils",
        "rs/types/wasm_types",
        "rs/universal_canister/impl",
        "rs/universal_canister/lib",
        "rs/utils",
        "rs/utils/ensure",
        "rs/utils/lru_cache",
        "rs/utils/rustfmt",
        "rs/utils/thread",
        "rs/validator",
        "rs/validator/fuzz",
        "rs/validator/http_request_arbitrary",
        "rs/validator/http_request_test_utils",
        "rs/validator/ingress_message",
        "rs/validator/ingress_message/test_canister",
        "rs/wasm_transform",
        "rs/workload_generator",
        "rs/xnet/endpoint",
        "rs/xnet/hyper",
        "rs/xnet/payload_builder",
        "rs/xnet/uri",
        "testnet",
        "testnet/tools",
        "third_party/bitcoin-core",
        "third_party/lmdb",
        "third_party/lmdb-rkv-0.14.99",
        "third_party/tlaplus-1.8.0",
        "toolchains/sysimage",
    ]

    mock = mocker.patch("release_notes.bazel_query")

    def bazel_query_side_effect(*args, **kwargs):
        if args[0] == "//...:*":
            return bazel_packages_all
        else:
            return guest_os_packages_all

    mock.side_effect = bazel_query_side_effect
    assert (
        prepare_release_notes(
            "release-2024-07-10_23-01-base",
            "a3831c87440df4821b435050c8a8fcb3745d86f6",
            "release-2024-07-25_21-03-base",
            "2c0b76cfc7e596d5c4304cff5222a2619294c8c1",
        )
        == """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

- @team-execution
- @team-messaging

# Release Notes for [release-2024-07-25_21-03-base](https://github.com/dfinity/ic/tree/release-2024-07-25_21-03-base) (`2c0b76cfc7e596d5c4304cff5222a2619294c8c1`)
This release is based on changes since [release-2024-07-10_23-01-base](https://dashboard.internetcomputer.org/release/a3831c87440df4821b435050c8a8fcb3745d86f6) (`a3831c87440df4821b435050c8a8fcb3745d86f6`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-07-10_23-01-base...release-2024-07-25_21-03-base).

This release diverges from latest release. Merge base is [6135fdcf35e8226a0ff11342d608e5a5abd24129](https://github.com/dfinity/ic/tree/6135fdcf35e8226a0ff11342d608e5a5abd24129).
Changes [were removed](https://github.com/dfinity/ic/compare/release-2024-07-25_21-03-base...release-2024-07-10_23-01-base) from this release.
## Features:
* author: Eero Kell | [`f5491f4b2`](https://github.com/dfinity/ic/commit/f5491f4b2) Consensus,Interface: Add backoff and jitter to HostOS upgrades ([#395](https://github.com/dfinity/ic/pull/395))
* author: Rost Rume | [`3ba4a08a2`](https://github.com/dfinity/ic/commit/3ba4a08a2) Crypto,Interface: quinn and rustls upgrade
* author: Drag Djur | [`2bae326f0`](https://github.com/dfinity/ic/commit/2bae326f0) Execution,Interface: Add new type of task OnLowWasmMemory ([#379](https://github.com/dfinity/ic/pull/379))
* author: Alex      | [`e7a36d5c8`](https://github.com/dfinity/ic/commit/e7a36d5c8) Execution,Interface: Handle canister snapshots during subnet splitting ([#412](https://github.com/dfinity/ic/pull/412))
* author: Dimi Sarl | [`59f22753b`](https://github.com/dfinity/ic/commit/59f22753b) Execution,Interface: Print instructions consumed in DTS executions in a more readable form
* author: Ulan Dege | [`9f25198cf`](https://github.com/dfinity/ic/commit/9f25198cf) Execution,Interface: Reland switch to compiler sandbox for compilation
* author: Alex Uta  | [`75c57bc48`](https://github.com/dfinity/ic/commit/75c57bc48) Execution,Interface,Networking: Adjust max number of cached sandboxes
* author: Dimi Sarl | [`9416ad7d0`](https://github.com/dfinity/ic/commit/9416ad7d0) Interface: Compute effective canister id for canister snapshot requests ([#541](https://github.com/dfinity/ic/pull/541))
* ~~author: Niko      | [`67e53cc29`](https://github.com/dfinity/ic/commit/67e53cc29) Interface(ICP-Rosetta): add rosetta blocks to block/transaction endpoint ([#524](https://github.com/dfinity/ic/pull/524)) [AUTO-EXCLUDED]~~
* ~~author: Andr Popo | [`fd0eafaf4`](https://github.com/dfinity/ic/commit/fd0eafaf4) Interface(sns): Include hash of upgrade args in UpgradeSnsControlledCanister payload text rendering ([#554](https://github.com/dfinity/ic/pull/554)) [AUTO-EXCLUDED]~~
* ~~author: dani      | [`871efb5cc`](https://github.com/dfinity/ic/commit/871efb5cc) Interface(nns): Added setting neuron visibility. ([#517](https://github.com/dfinity/ic/pull/517)) [AUTO-EXCLUDED]~~
* author: jaso      | [`b3ac41768`](https://github.com/dfinity/ic/commit/b3ac41768) Interface(nns): Support StopOrStartCanister proposal action ([#458](https://github.com/dfinity/ic/pull/458))
* ~~author: dani      | [`3625067d6`](https://github.com/dfinity/ic/commit/3625067d6) Interface(nns): Added visibility field to neurons. ([#451](https://github.com/dfinity/ic/pull/451)) [AUTO-EXCLUDED]~~
* author: Dani Shar | [`faa3c1ad8`](https://github.com/dfinity/ic/commit/faa3c1ad8) Interface(pocket-ic): Support synchronous call endpoint in pocket-ic. ([#348](https://github.com/dfinity/ic/pull/348))
* ~~author: jaso      | [`b8cd861b9`](https://github.com/dfinity/ic/commit/b8cd861b9) Interface: Add bitcoin and cycles ledger canisters to protocol canisters ([#424](https://github.com/dfinity/ic/pull/424)) [AUTO-EXCLUDED]~~
* ~~author: Niko Milo | [`215fb78b6`](https://github.com/dfinity/ic/commit/215fb78b6) Interface(farm): extending from config testnet ([#359](https://github.com/dfinity/ic/pull/359)) [AUTO-EXCLUDED]~~
* author: jaso      | [`922a89e6b`](https://github.com/dfinity/ic/commit/922a89e6b) Interface(nns): Create a new proposal action install_code and support non-root canisters ([#394](https://github.com/dfinity/ic/pull/394))
* ~~author: Mari Past | [`5ac0b1653`](https://github.com/dfinity/ic/commit/5ac0b1653) Interface: transaction uniqueness in Rosetta Blocks [AUTO-EXCLUDED]~~
* author: Igor Novg | [`fde205151`](https://github.com/dfinity/ic/commit/fde205151) Interface: ic-boundary: retry on most calls
* ~~author: Niko Haim | [`5bba7bd69`](https://github.com/dfinity/ic/commit/5bba7bd69) Interface(ICP-Rosetta): Add query block range [AUTO-EXCLUDED]~~
* ~~author: Jaso (Yel | [`891c74208`](https://github.com/dfinity/ic/commit/891c74208) Interface(nns): Create 2 new topics while not allowing following to be set on them [AUTO-EXCLUDED]~~
* author: Andr Popo | [`42fb959d5`](https://github.com/dfinity/ic/commit/42fb959d5) Interface(nns): Better field names for API type `NeuronsFundNeuronPortion`
* ~~author: Mari Past | [`a9d1d1052`](https://github.com/dfinity/ic/commit/a9d1d1052) Interface: support Rosetta Blocks in /blocks in icp rosetta [AUTO-EXCLUDED]~~
* author: Chri Stie | [`0f3b81c5f`](https://github.com/dfinity/ic/commit/0f3b81c5f) Interface,Message Routing: Implement handling reject signals from incoming stream slices.
* author: Alex Uta  | [`d267d7f0f`](https://github.com/dfinity/ic/commit/d267d7f0f) Interface,Message Routing,Networking: Revert to the memory allocator ([#515](https://github.com/dfinity/ic/pull/515))
* author: Tim  Gret | [`4c03f768f`](https://github.com/dfinity/ic/commit/4c03f768f) Interface,Networking: publish https outcalls adapter with http enabled for dfx
* author: Eero Kell | [`7d70776f8`](https://github.com/dfinity/ic/commit/7d70776f8) Interface,Node: Pull HostOS upgrade file in chunks
* ~~author: Nico Matt | [`aa89e8079`](https://github.com/dfinity/ic/commit/aa89e8079) IDX: Add Apple Silicon builds ([#512](https://github.com/dfinity/ic/pull/512)) [AUTO-EXCLUDED]~~
## Bugfixes:
* ~~author: Rost Rume | [`b239fb792`](https://github.com/dfinity/ic/commit/b239fb792) General: upgrade the bytes crate since v1.6.0 was yanked due to a bug [AUTO-EXCLUDED]~~
* author: Adri Alic | [`4fd343cae`](https://github.com/dfinity/ic/commit/4fd343cae) Consensus,Interface(consensus): Fix inconsistency when purging validated pool below maximum element ([#598](https://github.com/dfinity/ic/pull/598))
* author: Chri Müll | [`9243f5c75`](https://github.com/dfinity/ic/commit/9243f5c75) Consensus,Interface: ic-replay when DTS is enabled
* author: Jack Lloy | [`72e6f39b0`](https://github.com/dfinity/ic/commit/72e6f39b0) Crypto,Interface(crypto): Re-enable NIDKG cheating dealer solving test
* author: Nico Matt | [`3eb105c27`](https://github.com/dfinity/ic/commit/3eb105c27) Execution,Interface(IDX): remove unused aarch64 import ([#507](https://github.com/dfinity/ic/pull/507))
* author: Nico Matt | [`d1d720915`](https://github.com/dfinity/ic/commit/d1d720915) Execution,Interface(IDX): disable unused aarch64-darwin code ([#486](https://github.com/dfinity/ic/pull/486))
* author: Alex Uta  | [`ff9e2941c`](https://github.com/dfinity/ic/commit/ff9e2941c) Execution,Interface: Cap Wasm64 heap memory size ([#446](https://github.com/dfinity/ic/pull/446))
* author: Alex Uta  | [`d23960734`](https://github.com/dfinity/ic/commit/d23960734) Execution,Interface: Fix instrumentation for memory.init and table.init in Wasm 64-bit mode ([#442](https://github.com/dfinity/ic/pull/442))
* author: Ulan Dege | [`7708333b2`](https://github.com/dfinity/ic/commit/7708333b2) Execution,Interface: Follow up on the reserved cycles limit fix ([#383](https://github.com/dfinity/ic/pull/383))
* author: Ulan Dege | [`4a622c04c`](https://github.com/dfinity/ic/commit/4a622c04c) Execution,Interface: Free SandboxedExecutionController threads ([#354](https://github.com/dfinity/ic/pull/354))
* author: Andr Bere | [`587c1485b`](https://github.com/dfinity/ic/commit/587c1485b) Execution,Interface: Revert "feat: Switch to compiler sandbox for compilation"
* author: Stef Schn | [`fc5913c1c`](https://github.com/dfinity/ic/commit/fc5913c1c) Execution,Interface,Message Routing: Maintain snapshot_ids correctly ([#360](https://github.com/dfinity/ic/pull/360))
* author: Stef      | [`dd0be35cb`](https://github.com/dfinity/ic/commit/dd0be35cb) Interface: fifo tracing layers and connections dashboard ([#576](https://github.com/dfinity/ic/pull/576))
* author: max-      | [`994af8f87`](https://github.com/dfinity/ic/commit/994af8f87) Interface(registry): Optimize get_key_family ([#556](https://github.com/dfinity/ic/pull/556))
* author: Rost Rume | [`65c3775eb`](https://github.com/dfinity/ic/commit/65c3775eb) Interface: use idna for parsing domain names ([#414](https://github.com/dfinity/ic/pull/414))
* author: Luka Skug | [`2ef33c956`](https://github.com/dfinity/ic/commit/2ef33c956) Interface(k8s-testnets): adapt firewall rules for k8s testnets ([#436](https://github.com/dfinity/ic/pull/436))
* author: Bas  van  | [`3a31b54c3`](https://github.com/dfinity/ic/commit/3a31b54c3) Interface(IDX): double CPU reservation for //rs/nervous_system/integration_tests:integration_tests_test_tests/sns_ledger_upgrade ([#428](https://github.com/dfinity/ic/pull/428))
* author: Niko      | [`33187dbe8`](https://github.com/dfinity/ic/commit/33187dbe8) Interface: add e 8 s to icrc 21 ([#340](https://github.com/dfinity/ic/pull/340))
* ~~author: Niko      | [`18243444a`](https://github.com/dfinity/ic/commit/18243444a) Interface(ICRC-Index): remove comment on removing 0 balance accounts ([#341](https://github.com/dfinity/ic/pull/341)) [AUTO-EXCLUDED]~~
* author: Stef Schn | [`932506f89`](https://github.com/dfinity/ic/commit/932506f89) Interface,Message Routing: Add total_size to CanisterSnapshotBits ([#479](https://github.com/dfinity/ic/pull/479))
* author: Rost Rume | [`3ee248686`](https://github.com/dfinity/ic/commit/3ee248686) Interface,Networking: use the Shutdown struct instead of explicitly passing the cancellation token for the sender side of the consensus manager
* ~~author: Bas  van  | [`24278eb74`](https://github.com/dfinity/ic/commit/24278eb74) IDX: fix the did_git_test on GitHub ([#480](https://github.com/dfinity/ic/pull/480)) [AUTO-EXCLUDED]~~
* ~~author: Nico Matt | [`d7097b0ef`](https://github.com/dfinity/ic/commit/d7097b0ef) IDX: move build filters ([#482](https://github.com/dfinity/ic/pull/482)) [AUTO-EXCLUDED]~~
## Performance improvements:
* author: Leo  Eich | [`460693f61`](https://github.com/dfinity/ic/commit/460693f61) Consensus,Interface: Reduce cost of cloning tSchnorr inputs ([#344](https://github.com/dfinity/ic/pull/344))
* author: Jack Lloy | [`fac32ae6f`](https://github.com/dfinity/ic/commit/fac32ae6f) Crypto,Interface(crypto): Reduce the size of randomizers during Ed25519 batch verification ([#413](https://github.com/dfinity/ic/pull/413))
* author: Dimi Sarl | [`390135775`](https://github.com/dfinity/ic/commit/390135775) Execution,Interface: Speed up parsing of optional blob in CanisterHttpRequestArgs ([#478](https://github.com/dfinity/ic/pull/478))
## Chores:
* ~~author: r-bi      | [`af87b88ac`](https://github.com/dfinity/ic/commit/af87b88ac) General: bump response verification and associated crates ([#590](https://github.com/dfinity/ic/pull/590)) [AUTO-EXCLUDED]~~
* ~~author: Jack Lloy | [`72f9e6d7f`](https://github.com/dfinity/ic/commit/72f9e6d7f) General(crypto): Always optimize the curve25519-dalek crate [AUTO-EXCLUDED]~~
* author: Adri Alic | [`95f4680b0`](https://github.com/dfinity/ic/commit/95f4680b0) Consensus,Interface(consensus): Move get_block_maker_by_rank into test utilities ([#525](https://github.com/dfinity/ic/pull/525))
* author: Leo  Eich | [`1b4b3b478`](https://github.com/dfinity/ic/commit/1b4b3b478) Consensus,Interface: Update documentation to include tSchnorr ([#523](https://github.com/dfinity/ic/pull/523))
* author: Leo  Eich | [`282c6ec9c`](https://github.com/dfinity/ic/commit/282c6ec9c) Consensus,Interface: Rename `ecdsa` block payload field and fix comments ([#416](https://github.com/dfinity/ic/pull/416))
* author: Adri Alic | [`6ac0e1cce`](https://github.com/dfinity/ic/commit/6ac0e1cce) Consensus,Interface(consensus): Compute subnet members from membership directly ([#444](https://github.com/dfinity/ic/pull/444))
* author: Leo  Eich | [`2a530aa8f`](https://github.com/dfinity/ic/commit/2a530aa8f) Consensus,Interface: Rename `ecdsa` modules, `EcdsaClient`, `EcdsaGossip` and `EcdsaImpl` ([#367](https://github.com/dfinity/ic/pull/367))
* author: Leo  Eich | [`6057ce233`](https://github.com/dfinity/ic/commit/6057ce233) Consensus,Interface: Remove proto field used to migrate payload layout ([#380](https://github.com/dfinity/ic/pull/380))
* author: push      | [`1c78e64a0`](https://github.com/dfinity/ic/commit/1c78e64a0) Consensus,Interface(github-sync): PR#314 / fix(): ic-replay: do not try to verify the certification shares for heights below the CU
* author: Leo  Eich | [`99f80a4e6`](https://github.com/dfinity/ic/commit/99f80a4e6) Consensus,Interface: Rename `EcdsaPreSig*`, `EcdsaBlock*`, `EcdsaTranscript*`, and `EcdsaSig*`
* author: Leo  Eich | [`b13539c23`](https://github.com/dfinity/ic/commit/b13539c23) Consensus,Interface: Rename `EcdsaPayload`
* author: push      | [`f906cf8da`](https://github.com/dfinity/ic/commit/f906cf8da) Crypto(github-sync): PR#248 / feat(crypto): add new signature verification package initially supporting canister signatures
* author: Jack Lloy | [`dbaa4375c`](https://github.com/dfinity/ic/commit/dbaa4375c) Crypto,Interface(crypto): Remove support for masked kappa in threshold ECDSA ([#368](https://github.com/dfinity/ic/pull/368))
* author: Jack Lloy | [`bed4f13ef`](https://github.com/dfinity/ic/commit/bed4f13ef) Crypto,Interface(crypto): Implement ZIP25 Ed25519 verification in ic_crypto_ed25519
* author: Dimi Sarl | [`d1206f45a`](https://github.com/dfinity/ic/commit/d1206f45a) Execution,Interface: Add logs to capture usages of legacy ICQC feature on system subnets ([#607](https://github.com/dfinity/ic/pull/607))
* author: Dimi Sarl | [`bc2755cff`](https://github.com/dfinity/ic/commit/bc2755cff) Execution,Interface(execution): Remove wasm_chunk_store flag ([#542](https://github.com/dfinity/ic/pull/542))
* author: Maks Arut | [`7a8c6c69f`](https://github.com/dfinity/ic/commit/7a8c6c69f) Execution,Interface: unify ECDSA and tSchnorr signing requests ([#544](https://github.com/dfinity/ic/pull/544))
* author: Dimi Sarl | [`513b2baec`](https://github.com/dfinity/ic/commit/513b2baec) Execution,Interface(management-canister): Remove unimplemented delete_chunks API ([#537](https://github.com/dfinity/ic/pull/537))
* author: Maks Arut | [`e41aefe34`](https://github.com/dfinity/ic/commit/e41aefe34) Execution,Interface: remove obsolete canister_logging feature flag ([#505](https://github.com/dfinity/ic/pull/505))
* author: Dimi Sarl | [`005885513`](https://github.com/dfinity/ic/commit/005885513) Execution,Interface: Remove deprecated controller field in update settings requests ([#432](https://github.com/dfinity/ic/pull/432))
* author: Ulan Dege | [`45aefaf9f`](https://github.com/dfinity/ic/commit/45aefaf9f) Execution,Interface: Derive ParitalEq for all sandbox IPC types ([#374](https://github.com/dfinity/ic/pull/374))
* author: Andr Bere | [`234e5c396`](https://github.com/dfinity/ic/commit/234e5c396) Execution,Interface: Update Wasm benchmarks
* author: Maks Arut | [`2411eb905`](https://github.com/dfinity/ic/commit/2411eb905) Execution,Interface: rename iDKG key to threshold key
* author: Dimi Sarl | [`1ba3b5e0b`](https://github.com/dfinity/ic/commit/1ba3b5e0b) Execution,Interface,Message Routing: Update error message for subnet methods that are not allowed through ingress messages ([#574](https://github.com/dfinity/ic/pull/574))
* author: Venk Seka | [`5dc3afeb5`](https://github.com/dfinity/ic/commit/5dc3afeb5) Execution,Interface,Networking(fuzzing): fix clippy warnings for fuzzers
* ~~author: maci      | [`3ecb66f20`](https://github.com/dfinity/ic/commit/3ecb66f20) Interface(ICP/ICRC-ledger): return value in BalanceStrore.get_balance ([#518](https://github.com/dfinity/ic/pull/518)) [AUTO-EXCLUDED]~~
* author: Dimi Sarl | [`c4eb29da7`](https://github.com/dfinity/ic/commit/c4eb29da7) Interface: Remove unused instruction limits from subnet record ([#441](https://github.com/dfinity/ic/pull/441))
* ~~author: Niko      | [`cec100d16`](https://github.com/dfinity/ic/commit/cec100d16) Interface(ICRC-Rosetta): add secp key test ([#467](https://github.com/dfinity/ic/pull/467)) [AUTO-EXCLUDED]~~
* author: Maks Arut | [`eec6107fa`](https://github.com/dfinity/ic/commit/eec6107fa) Interface: remove obsolete cost scaling feature flag ([#502](https://github.com/dfinity/ic/pull/502))
* ~~author: Niko      | [`6764190a8`](https://github.com/dfinity/ic/commit/6764190a8) Interface(ICP-Rosetta): enable feature flag rosetta blocks ([#465](https://github.com/dfinity/ic/pull/465)) [AUTO-EXCLUDED]~~
* ~~author: maci      | [`14836b59d`](https://github.com/dfinity/ic/commit/14836b59d) Interface(ICP/ICRC-Ledger): refactor approvals library to allow using regular and stable allowance storage ([#382](https://github.com/dfinity/ic/pull/382)) [AUTO-EXCLUDED]~~
* author: Rost Rume | [`4cc989aa3`](https://github.com/dfinity/ic/commit/4cc989aa3) Interface: upgrade url and uuid and use workspace versions ([#417](https://github.com/dfinity/ic/pull/417))
* ~~author: max-      | [`d732d9d6d`](https://github.com/dfinity/ic/commit/d732d9d6d) Interface(nns): Add api <--> internal type conversions ([#393](https://github.com/dfinity/ic/pull/393)) [AUTO-EXCLUDED]~~
* ~~author: r-bi      | [`9a3aa19d7`](https://github.com/dfinity/ic/commit/9a3aa19d7) Interface(ic-boundary): removing deprecated CLI option ([#404](https://github.com/dfinity/ic/pull/404)) [AUTO-EXCLUDED]~~
* author: mras      | [`3ba594f48`](https://github.com/dfinity/ic/commit/3ba594f48) Interface: collection of preparatory steps for canister HTTP outcalls in PocketIC and unrelated fixes ([#352](https://github.com/dfinity/ic/pull/352))
* author: Rost Rume | [`c52bf40a1`](https://github.com/dfinity/ic/commit/c52bf40a1) Interface: upgrade rustls
* author: Rost Rume | [`5cfaea5ea`](https://github.com/dfinity/ic/commit/5cfaea5ea) Interface: upgrade external crates and use workspace version
* author: Ognj Mari | [`3017e2e4a`](https://github.com/dfinity/ic/commit/3017e2e4a) Interface: move some Bazel rules out of the system test defs
* author: Stef Neam | [`0a9901ae4`](https://github.com/dfinity/ic/commit/0a9901ae4) Interface: remove old hyper from system tests
* ~~author: Andr Popo | [`91ceadc58`](https://github.com/dfinity/ic/commit/91ceadc58) Interface,Message Routing(nervous_system): Principals proto typo fix: 7 -> 1 ([#375](https://github.com/dfinity/ic/pull/375)) [AUTO-EXCLUDED]~~
* author: mras      | [`11bc5648c`](https://github.com/dfinity/ic/commit/11bc5648c) Interface,Networking: publish ic-https-outcalls-adapter-https-only ([#578](https://github.com/dfinity/ic/pull/578))
* author: Dani Shar | [`deafb0a12`](https://github.com/dfinity/ic/commit/deafb0a12) Interface,Networking(http-endpoint): Increase `SETTINGS_MAX_CONCURRENT_STREAMS` to 1000 ([#349](https://github.com/dfinity/ic/pull/349))
* author: Tim  Gret | [`0775cd819`](https://github.com/dfinity/ic/commit/0775cd819) Interface,Networking: abort artifact download externally if peer set is empty
* author: Stef Neam | [`a91bae41e`](https://github.com/dfinity/ic/commit/a91bae41e) Interface,Networking: decompress bitcoin data inside tests
* author: Dani Shar | [`b2268cbaa`](https://github.com/dfinity/ic/commit/b2268cbaa) Interface,Networking(ingress-watcher): Add metric to track capacity of the channel from execeution
* author: Rost Rume | [`3d1337795`](https://github.com/dfinity/ic/commit/3d1337795) Interface,Node: make the visibility rules consistent ([#567](https://github.com/dfinity/ic/pull/567))
* author: Rost Rume | [`21c75cb41`](https://github.com/dfinity/ic/commit/21c75cb41) Interface,Node: introduce release-pkg and ic-os-pkg package groups ([#553](https://github.com/dfinity/ic/pull/553))
* author: r-bi      | [`eb775492d`](https://github.com/dfinity/ic/commit/eb775492d) Interface,Node: firewall counter exporter ([#343](https://github.com/dfinity/ic/pull/343))
* ~~author: Mark Kosm | [`2c0b76cfc`](https://github.com/dfinity/ic/commit/2c0b76cfc) IDX: updating container autobuild ([#390](https://github.com/dfinity/ic/pull/390)) [AUTO-EXCLUDED]~~
* ~~author: Luka Skug | [`7c5e06583`](https://github.com/dfinity/ic/commit/7c5e06583) IDX: revert "remove binaries which don't need to be released (e.g. stripped) and don't need to to uploaded to the CDN" ([#616](https://github.com/dfinity/ic/pull/616)) [AUTO-EXCLUDED]~~
* ~~author: Rost Rume | [`fd136861c`](https://github.com/dfinity/ic/commit/fd136861c) IDX: don't not upload/compress test canisters ([#561](https://github.com/dfinity/ic/pull/561)) [AUTO-EXCLUDED]~~
* ~~author: Rost Rume | [`9c36d497b`](https://github.com/dfinity/ic/commit/9c36d497b) IDX: remove binaries which don't need to be released (e.g. stripped) and don't need to to uploaded to the CDN ([#563](https://github.com/dfinity/ic/pull/563)) [AUTO-EXCLUDED]~~
* ~~author: Nico Matt | [`3ea305b03`](https://github.com/dfinity/ic/commit/3ea305b03) IDX: remove targets from rust_canister ([#440](https://github.com/dfinity/ic/pull/440)) [AUTO-EXCLUDED]~~
* ~~author: Nico Matt | [`c5121e693`](https://github.com/dfinity/ic/commit/c5121e693) IDX: split .bazelrc ([#459](https://github.com/dfinity/ic/pull/459)) [AUTO-EXCLUDED]~~
* author: sa-g      | [`1999421a1`](https://github.com/dfinity/ic/commit/1999421a1) Node: Update Base Image Refs [2024-07-25-0808] ([#601](https://github.com/dfinity/ic/pull/601))
* author: sa-g      | [`c488577bc`](https://github.com/dfinity/ic/commit/c488577bc) Node: Update Base Image Refs [2024-07-20-0145] ([#492](https://github.com/dfinity/ic/pull/492))
* author: sa-g      | [`52b65a8af`](https://github.com/dfinity/ic/commit/52b65a8af) Node: Update Base Image Refs [2024-07-17-0147] ([#397](https://github.com/dfinity/ic/pull/397))
* author: Andr Batt | [`3aae377ca`](https://github.com/dfinity/ic/commit/3aae377ca) Node: Log HostOS config partition (config.ini and deployment.json)
* author: DFIN GitL | [`233657b46`](https://github.com/dfinity/ic/commit/233657b46) Node: Update container base images refs [2024-07-12-0623]
## Refactoring:
* author: Rost Rume | [`e21c3e74e`](https://github.com/dfinity/ic/commit/e21c3e74e) Consensus,Interface,Networking: move the PriorityFn under interfaces and rename the PrioriyFnAndFilterProducer to PriorityFnFactory
* author: Fran Prei | [`5b8fc4237`](https://github.com/dfinity/ic/commit/5b8fc4237) Crypto,Interface(crypto): remove CspPublicAndSecretKeyStoreChecker ([#559](https://github.com/dfinity/ic/pull/559))
* author: Fran Prei | [`63da4b23a`](https://github.com/dfinity/ic/commit/63da4b23a) Crypto,Interface(crypto): unify threshold sign method names ([#321](https://github.com/dfinity/ic/pull/321))
* author: Fran Prei | [`1413afe92`](https://github.com/dfinity/ic/commit/1413afe92) Crypto,Interface(crypto): replace ed25519-consensus with ic-crypto-ed25519 in prod ([#347](https://github.com/dfinity/ic/pull/347))
* author: Venk Seka | [`34ff2857a`](https://github.com/dfinity/ic/commit/34ff2857a) Execution,Interface(fuzzing): create new test library `wasm_fuzzers`
* author: stie      | [`61870cc77`](https://github.com/dfinity/ic/commit/61870cc77) Execution,Interface,Message Routing: Remove misleading `callback_id` from `register_callback()` test function ([#497](https://github.com/dfinity/ic/pull/497))
* ~~author: Math Björ | [`2e8fa1ad7`](https://github.com/dfinity/ic/commit/2e8fa1ad7) Interface(icp_ledger): Move test helper functions to test utils ([#462](https://github.com/dfinity/ic/pull/462)) [AUTO-EXCLUDED]~~
* ~~author: max-      | [`d04d4bbd5`](https://github.com/dfinity/ic/commit/d04d4bbd5) Interface(nns): no longer generate api types from internal protos ([#588](https://github.com/dfinity/ic/pull/588)) [AUTO-EXCLUDED]~~
* ~~author: max-      | [`2926051d5`](https://github.com/dfinity/ic/commit/2926051d5) Interface(nns): Move governance::init to its own crate to further split type dependencies ([#490](https://github.com/dfinity/ic/pull/490)) [AUTO-EXCLUDED]~~
* author: Andr Popo | [`a7f5db70e`](https://github.com/dfinity/ic/commit/a7f5db70e) Interface(nervous_system): Add `controller` and `hotkeys` fields to CfParticipant, CfNeuron, and CfInvestment ([#373](https://github.com/dfinity/ic/pull/373))
* ~~author: max-      | [`d0a0cc72a`](https://github.com/dfinity/ic/commit/d0a0cc72a) Interface(nns): Use governance_api instead of governance types in entrypoint in governance ([#457](https://github.com/dfinity/ic/pull/457)) [AUTO-EXCLUDED]~~
* author: Andr Popo | [`8a852bed9`](https://github.com/dfinity/ic/commit/8a852bed9) Interface(nervous_system): Move `Principals` message definition to nervous_system/proto ([#447](https://github.com/dfinity/ic/pull/447))
* ~~author: Andr Popo | [`7d3245ce7`](https://github.com/dfinity/ic/commit/7d3245ce7) Interface(nervous_system): Add fields with better names to NeuronsFundNeuron [AUTO-EXCLUDED]~~
* author: tim  gret | [`f3628917c`](https://github.com/dfinity/ic/commit/f3628917c) Interface,Networking: introduce artifact downloader component ([#403](https://github.com/dfinity/ic/pull/403))
## Tests:
* author: Ulan Dege | [`e15d65e1c`](https://github.com/dfinity/ic/commit/e15d65e1c) Execution,Interface: Add execution smoke tests ([#526](https://github.com/dfinity/ic/pull/526))
* author: Ulan Dege | [`ba82afe4d`](https://github.com/dfinity/ic/commit/ba82afe4d) Execution,Interface: Add unit tests for sandbox to replica IPC messages ([#435](https://github.com/dfinity/ic/pull/435))
* author: Ulan Dege | [`9552f0828`](https://github.com/dfinity/ic/commit/9552f0828) Execution,Interface: Add unit tests for replica to sandbox IPC messages ([#411](https://github.com/dfinity/ic/pull/411))
* author: Drag Djur | [`de3425fa6`](https://github.com/dfinity/ic/commit/de3425fa6) Execution,Interface: make system api test to be state machine test ([#377](https://github.com/dfinity/ic/pull/377))
* author: Maks Arut | [`c12b4b26d`](https://github.com/dfinity/ic/commit/c12b4b26d) Execution,Interface: support signing disabled iDKG keys in state_machine_tests
* ~~author: Ulan Dege | [`bc8db7683`](https://github.com/dfinity/ic/commit/bc8db7683) Interface: Remove the scalability benchmarking suite ([#527](https://github.com/dfinity/ic/pull/527)) [AUTO-EXCLUDED]~~
* ~~author: Math Björ | [`f2f408333`](https://github.com/dfinity/ic/commit/f2f408333) Interface(ICRC-Ledger): Add tests for upgrading ICRC ledger with WASMs with different token types ([#388](https://github.com/dfinity/ic/pull/388)) [AUTO-EXCLUDED]~~
* ~~author: Math Björ | [`620613591`](https://github.com/dfinity/ic/commit/620613591) Interface(icrc_ledger): Upgrade test for ledgers using golden state ([#399](https://github.com/dfinity/ic/pull/399)) [AUTO-EXCLUDED]~~
* author: dani      | [`2d2f3b550`](https://github.com/dfinity/ic/commit/2d2f3b550) Interface(sns): SNS upgrade-related tests were flaking out. ([#391](https://github.com/dfinity/ic/pull/391))
* author: Ognj Mari | [`38c7a5098`](https://github.com/dfinity/ic/commit/38c7a5098) Interface,Message Routing: check canister queue upgrade/downgrade compatibility against published version
* ~~author: Rost Rume | [`3e4a107f6`](https://github.com/dfinity/ic/commit/3e4a107f6) IDX: stop uploading test canister artifacts  ([#533](https://github.com/dfinity/ic/pull/533)) [AUTO-EXCLUDED]~~
## Documentation:
* ~~author: Rost Rume | [`7c4a08fc2`](https://github.com/dfinity/ic/commit/7c4a08fc2) General: why GuestOS deps are required ([#410](https://github.com/dfinity/ic/pull/410)) [AUTO-EXCLUDED]~~
* ~~author: Andr Popo | [`16dc659a0`](https://github.com/dfinity/ic/commit/16dc659a0) Interface(sns): Typo fix ManageVotingPermissions → ManageVotingPermission [AUTO-EXCLUDED]~~
## Other changes:
* ~~author: Math Björ | [`364fe4f38`](https://github.com/dfinity/ic/commit/364fe4f38) Interface: test(icp_ledger):, Get and query all blocks from ledger and archives and fix test_archive_indexing ([#398](https://github.com/dfinity/ic/pull/398)) [AUTO-EXCLUDED]~~
* author: Dani Wong | [`15beeb6a9`](https://github.com/dfinity/ic/commit/15beeb6a9) Interface(nns): Add and use workspace version of prometheus-parse.
"""
    )

    assert (
        prepare_release_notes(
            "release-2024-07-25_21-03-base",
            "2c0b76cfc7e596d5c4304cff5222a2619294c8c1",
            "release-2024-08-02_01-30-base",
            "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d",
        )
        == """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

- @team-execution
- @team-messaging

# Release Notes for [release-2024-08-02_01-30-base](https://github.com/dfinity/ic/tree/release-2024-08-02_01-30-base) (`3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d`)
This release is based on changes since [release-2024-07-25_21-03-base](https://dashboard.internetcomputer.org/release/2c0b76cfc7e596d5c4304cff5222a2619294c8c1) (`2c0b76cfc7e596d5c4304cff5222a2619294c8c1`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-07-25_21-03-base...release-2024-08-02_01-30-base).
## Features:
* author: Adri Alic | [`5e319b9de`](https://github.com/dfinity/ic/commit/5e319b9de) Consensus,Interface(consensus): Change definition of better to exclude disqualified block makers ([#673](https://github.com/dfinity/ic/pull/673))
* author: Alex Uta  | [`736beea98`](https://github.com/dfinity/ic/commit/736beea98) Execution,Interface,Message Routing,Runtime: Enable transparent huge pages for the page allocator ([#665](https://github.com/dfinity/ic/pull/665))
* author: Dimi Sarl | [`96035ca4c`](https://github.com/dfinity/ic/commit/96035ca4c) Execution,Interface,Networking,Runtime: Reduce DTS slice limit for regular messages on system subnets ([#621](https://github.com/dfinity/ic/pull/621))
* author: Alex      | [`f0093242d`](https://github.com/dfinity/ic/commit/f0093242d) Execution,Interface,Runtime: Enforce taking a canister snapshot only when canister is not empty ([#452](https://github.com/dfinity/ic/pull/452))
* ~~author: maci      | [`9397d7264`](https://github.com/dfinity/ic/commit/9397d7264) Financial Integrations(icrc-ledger-types): bumping version to 0.1.6 in order to release icrc3 and icrc21 types. ([#509](https://github.com/dfinity/ic/pull/509)) [AUTO-EXCLUDED]~~
* ~~author: dani      | [`a89a2e17c`](https://github.com/dfinity/ic/commit/a89a2e17c) Interface(nns): Metrics for public neurons. ([#685](https://github.com/dfinity/ic/pull/685)) [AUTO-EXCLUDED]~~
* author: dani      | [`448c85ccc`](https://github.com/dfinity/ic/commit/448c85ccc) Interface(nns): Added include_public_neurons_in_full_neurons to ListNeurons. ([#589](https://github.com/dfinity/ic/pull/589))
* ~~author: jaso      | [`2b109fb9b`](https://github.com/dfinity/ic/commit/2b109fb9b) Interface(nns): Define update_canister_settings proposal type without execution ([#529](https://github.com/dfinity/ic/pull/529)) [AUTO-EXCLUDED]~~
## Bugfixes:
* author: Adri Alic | [`2bdfdc54c`](https://github.com/dfinity/ic/commit/2bdfdc54c) Consensus,Interface(consensus): Use correct signer id in make_next_block_with_rank ([#644](https://github.com/dfinity/ic/pull/644))
* ~~author: r-bi      | [`d5a950484`](https://github.com/dfinity/ic/commit/d5a950484) Interface(ic-boundary): switch logging setup from eager to lazy eval ([#658](https://github.com/dfinity/ic/pull/658)) [AUTO-EXCLUDED]~~
* ~~author: Andr Popo | [`395c0e49a`](https://github.com/dfinity/ic/commit/395c0e49a) Interface(sns): Enforce a minimum on the maximum number of permissioned principals an SNS neuron is allowed to have ([#649](https://github.com/dfinity/ic/pull/649)) [AUTO-EXCLUDED]~~
* author: Dimi Sarl | [`9fc5fc83f`](https://github.com/dfinity/ic/commit/9fc5fc83f) Interface: Update computation of effective canister id for FetchCanisterLogs ([#540](https://github.com/dfinity/ic/pull/540))
* ~~author: Bas  van  | [`0efbeeb91`](https://github.com/dfinity/ic/commit/0efbeeb91) IDX: only run system_test_benchmark tests when targeted explicitly ([#693](https://github.com/dfinity/ic/pull/693)) [AUTO-EXCLUDED]~~
* ~~author: Rost Rume | [`fd7fc6ebe`](https://github.com/dfinity/ic/commit/fd7fc6ebe) IDX: fix our release rules ([#630](https://github.com/dfinity/ic/pull/630)) [AUTO-EXCLUDED]~~
## Chores:
* author: kpop      | [`204542c15`](https://github.com/dfinity/ic/commit/204542c15) Consensus,Interface(consensus): change the associated `Error` type of `TryFrom<pb>` from `String` to `ProxyDecodeError` for some consensus types ([#695](https://github.com/dfinity/ic/pull/695))
* author: Drag Djur | [`4bebd6f6a`](https://github.com/dfinity/ic/commit/4bebd6f6a) Execution,Interface: Add Wasm memory threshold field to canister settings ([#475](https://github.com/dfinity/ic/pull/475))
* author: Ulan Dege | [`3e9785f87`](https://github.com/dfinity/ic/commit/3e9785f87) Execution,Interface,Runtime: Rename fees_and_limits to icp_config ([#638](https://github.com/dfinity/ic/pull/638))
* ~~author: Andr Popo | [`9bc6e18ac`](https://github.com/dfinity/ic/commit/9bc6e18ac) Interface(neurons_fund): Populate hotkeys when necessary in the NNS Governance → Swap → SNS Governance dataflow ([#688](https://github.com/dfinity/ic/pull/688)) [AUTO-EXCLUDED]~~
* author: Dani Shar | [`b4be567dc`](https://github.com/dfinity/ic/commit/b4be567dc) Interface: Bump rust version to 1.80 ([#642](https://github.com/dfinity/ic/pull/642))
* author: mras      | [`dbfbeceea`](https://github.com/dfinity/ic/commit/dbfbeceea) Interface: bump jemallocator v0.3 to tikv-jemallocator v0.5 ([#654](https://github.com/dfinity/ic/pull/654))
* author: Leo  Eich | [`668fbe08f`](https://github.com/dfinity/ic/commit/668fbe08f) Interface: Rename ECDSA metrics ([#535](https://github.com/dfinity/ic/pull/535))
* ~~author: Dani Shar | [`219655bf7`](https://github.com/dfinity/ic/commit/219655bf7) Interface: Update `agent-rs` dependency version to 0.37.1 ([#560](https://github.com/dfinity/ic/pull/560)) [AUTO-EXCLUDED]~~
* author: Rost Rume | [`ec01b3735`](https://github.com/dfinity/ic/commit/ec01b3735) Interface: add tools-pkg ([#584](https://github.com/dfinity/ic/pull/584))
* author: Dimi Sarl | [`0527e6f50`](https://github.com/dfinity/ic/commit/0527e6f50) Interface,Message Routing: Use a single sentence for error messages in IngressInductionError ([#648](https://github.com/dfinity/ic/pull/648))
* author: Rost Rume | [`173d06185`](https://github.com/dfinity/ic/commit/173d06185) Interface,Node: build and strip IC-OS tools iff we build the VMs ([#609](https://github.com/dfinity/ic/pull/609))
* author: Maci Kot  | [`f6a88d1a5`](https://github.com/dfinity/ic/commit/f6a88d1a5) Interface,Runtime: Saturate function index in system api calls ([#641](https://github.com/dfinity/ic/pull/641))
* author: sa-g      | [`c77043f06`](https://github.com/dfinity/ic/commit/c77043f06) Node: Update Base Image Refs [2024-08-01-0150] ([#712](https://github.com/dfinity/ic/pull/712))
* author: sa-g      | [`2c8adf74b`](https://github.com/dfinity/ic/commit/2c8adf74b) Node: Update Base Image Refs [2024-07-31-0139] ([#690](https://github.com/dfinity/ic/pull/690))
## Refactoring:
* author: kpop      | [`962bb3848`](https://github.com/dfinity/ic/commit/962bb3848) Consensus,Interface(consensus): clean up the `dkg::payload_validator` code a bit and increase the test coverage ([#661](https://github.com/dfinity/ic/pull/661))
* author: Fran Prei | [`9ff9f96b0`](https://github.com/dfinity/ic/commit/9ff9f96b0) Crypto,Interface(crypto): remove CspTlsHandshakeSignerProvider ([#627](https://github.com/dfinity/ic/pull/627))
* author: Fran Prei | [`1909c13a8`](https://github.com/dfinity/ic/commit/1909c13a8) Crypto,Interface(crypto): remove CspPublicKeyStore ([#625](https://github.com/dfinity/ic/pull/625))
* author: Andr Popo | [`96bc27800`](https://github.com/dfinity/ic/commit/96bc27800) Interface(sns): Add controller and hotkeys information to ClaimSwapNeuronsRequest, and use it in SNS Governance ([#596](https://github.com/dfinity/ic/pull/596))
* ~~author: Andr Popo | [`1a0c97fe4`](https://github.com/dfinity/ic/commit/1a0c97fe4) Interface(sns): Remove the open method from swap. [override-didc-check] ([#454](https://github.com/dfinity/ic/pull/454)) [AUTO-EXCLUDED]~~
* author: Dimi Sarl | [`50857b09e`](https://github.com/dfinity/ic/commit/50857b09e) Interface,Message Routing: Move IngressInductionError outside of replicated state ([#618](https://github.com/dfinity/ic/pull/618))
## Tests:
* author: Dimi Sarl | [`0ed8c497c`](https://github.com/dfinity/ic/commit/0ed8c497c) Consensus,Execution,Interface: Fix property tests in bitcoin consensus payload builder ([#656](https://github.com/dfinity/ic/pull/656))
"""
    )
