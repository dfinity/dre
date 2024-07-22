from release_notes import prepare_release_notes


def test_release_notes():
    assert (
        prepare_release_notes(
            "a3831c87440df4821b435050c8a8fcb3745d86f6",
            "de29a1a55b589428d173b31cdb8cec0923245657",
            "release-2024-07-18_01-30--github-base",
        )
        == """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

- @team-execution
- @team-messaging

# Release Notes for [release-2024-07-18_01-30--github-base](https://github.com/dfinity/ic/tree/release-2024-07-18_01-30--github-base) (de29a1a55b589428d173b31cdb8cec0923245657)
Changelog since git revision [a3831c87440df4821b435050c8a8fcb3745d86f6](https://dashboard.internetcomputer.org/release/a3831c87440df4821b435050c8a8fcb3745d86f6)
## Features:
* ~~author: Igor Novg | [`fde205151`](https://github.com/dfinity/ic/commit/fde205151) Boundary Nodes: ic-boundary: retry on most calls [AUTO-EXCLUDED]~~
* author: Eero Kell | [`f5491f4b2`](https://github.com/dfinity/ic/commit/f5491f4b2) Consensus: Add backoff and jitter to HostOS upgrades (#395)
* author: Rost Rume | [`3ba4a08a2`](https://github.com/dfinity/ic/commit/3ba4a08a2) crypto-team,Networking: quinn and rustls upgrade
* author: Dimi Sarl | [`59f22753b`](https://github.com/dfinity/ic/commit/59f22753b) Execution,Runtime: Print instructions consumed in DTS executions in a more readable form
* ~~author: Mari Past | [`5ac0b1653`](https://github.com/dfinity/ic/commit/5ac0b1653) Financial Integrations: transaction uniqueness in Rosetta Blocks [AUTO-EXCLUDED]~~
* ~~author: Niko Haim | [`5bba7bd69`](https://github.com/dfinity/ic/commit/5bba7bd69) Financial Integrations(ICP-Rosetta): Add query block range [AUTO-EXCLUDED]~~
* ~~author: Mari Past | [`a9d1d1052`](https://github.com/dfinity/ic/commit/a9d1d1052) Financial Integrations: support Rosetta Blocks in /blocks in icp rosetta [AUTO-EXCLUDED]~~
* author: Chri Stie | [`0f3b81c5f`](https://github.com/dfinity/ic/commit/0f3b81c5f) Message Routing: Implement handling reject signals from incoming stream slices.
* author: Tim  Gret | [`4c03f768f`](https://github.com/dfinity/ic/commit/4c03f768f) Networking: publish https outcalls adapter with http enabled for dfx
* ~~author: jaso      | [`922a89e6b`](https://github.com/dfinity/ic/commit/922a89e6b) NNS: Create a new proposal action install_code and support non-root canisters (#394) [AUTO-EXCLUDED]~~
* ~~author: Jaso (Yel | [`891c74208`](https://github.com/dfinity/ic/commit/891c74208) NNS: Create 2 new topics while not allowing following to be set on them [AUTO-EXCLUDED]~~
* ~~author: Andr Popo | [`42fb959d5`](https://github.com/dfinity/ic/commit/42fb959d5) NNS: Better field names for API type `NeuronsFundNeuronPortion` [AUTO-EXCLUDED]~~
* author: Eero Kell | [`7d70776f8`](https://github.com/dfinity/ic/commit/7d70776f8) Node: Pull HostOS upgrade file in chunks
* author: Alex Uta  | [`75c57bc48`](https://github.com/dfinity/ic/commit/75c57bc48) Runtime: Adjust max number of cached sandboxes
* author: Ulan Dege | [`9f25198cf`](https://github.com/dfinity/ic/commit/9f25198cf) Runtime: Reland switch to compiler sandbox for compilation
## Bugfixes:
* ~~author: Rost Rume | [`b239fb792`](https://github.com/dfinity/ic/commit/b239fb792) General: upgrade the bytes crate since v1.6.0 was yanked due to a bug [AUTO-EXCLUDED]~~
* author: Chri Müll | [`9243f5c75`](https://github.com/dfinity/ic/commit/9243f5c75) Consensus: ic-replay when DTS is enabled
* author: Ulan Dege | [`7708333b2`](https://github.com/dfinity/ic/commit/7708333b2) Execution,Runtime: Follow up on the reserved cycles limit fix (#383)
* ~~author: Niko      | [`18243444a`](https://github.com/dfinity/ic/commit/18243444a) Financial Integrations(ICRC-Index): remove comment on removing 0 balance accounts (#341) [AUTO-EXCLUDED]~~
* author: Rost Rume | [`3ee248686`](https://github.com/dfinity/ic/commit/3ee248686) Networking: use the Shutdown struct instead of explicitly passing the cancellation token for the sender side of the consensus manager
* author: Ulan Dege | [`4a622c04c`](https://github.com/dfinity/ic/commit/4a622c04c) Runtime: Free SandboxedExecutionController threads (#354)
* author: Andr Bere | [`587c1485b`](https://github.com/dfinity/ic/commit/587c1485b) Runtime: Revert "feat: Switch to compiler sandbox for compilation"
## Performance improvements:
* author: Leo  Eich | [`460693f61`](https://github.com/dfinity/ic/commit/460693f61) Consensus,Execution,Runtime: Reduce cost of cloning tSchnorr inputs (#344)
* author: Jack Lloy | [`fac32ae6f`](https://github.com/dfinity/ic/commit/fac32ae6f) crypto-team(crypto): Reduce the size of randomizers during Ed25519 batch verification (#413)
## Chores:
* ~~author: Jack Lloy | [`72f9e6d7f`](https://github.com/dfinity/ic/commit/72f9e6d7f) General(crypto): Always optimize the curve25519-dalek crate [AUTO-EXCLUDED]~~
* ~~author: r-bi      | [`9a3aa19d7`](https://github.com/dfinity/ic/commit/9a3aa19d7) Boundary Nodes(ic-boundary): removing deprecated CLI option (#404) [AUTO-EXCLUDED]~~
* author: Rost Rume | [`c52bf40a1`](https://github.com/dfinity/ic/commit/c52bf40a1) Boundary Nodes,IDX,Networking: upgrade rustls
* author: Rost Rume | [`5cfaea5ea`](https://github.com/dfinity/ic/commit/5cfaea5ea) Boundary Nodes,IDX,NNS,Node,pocket-ic: upgrade external crates and use workspace version
* author: Leo  Eich | [`2a530aa8f`](https://github.com/dfinity/ic/commit/2a530aa8f) Consensus: Rename `ecdsa` modules, `EcdsaClient`, `EcdsaGossip` and `EcdsaImpl` (#367)
* author: push      | [`1c78e64a0`](https://github.com/dfinity/ic/commit/1c78e64a0) Consensus(github-sync): PR#314 / fix(): ic-replay: do not try to verify the certification shares for heights below the CU
* author: Leo  Eich | [`99f80a4e6`](https://github.com/dfinity/ic/commit/99f80a4e6) Consensus: Rename `EcdsaPreSig*`, `EcdsaBlock*`, `EcdsaTranscript*`, and `EcdsaSig*`
* author: Leo  Eich | [`b13539c23`](https://github.com/dfinity/ic/commit/b13539c23) Consensus: Rename `EcdsaPayload`
* author: Leo  Eich | [`6057ce233`](https://github.com/dfinity/ic/commit/6057ce233) Consensus,Interface: Remove proto field used to migrate payload layout (#380)
* author: Jack Lloy | [`dbaa4375c`](https://github.com/dfinity/ic/commit/dbaa4375c) crypto-team(crypto): Remove support for masked kappa in threshold ECDSA (#368)
* author: Jack Lloy | [`bed4f13ef`](https://github.com/dfinity/ic/commit/bed4f13ef) crypto-team(crypto): Implement ZIP25 Ed25519 verification in ic_crypto_ed25519
* author: Andr Bere | [`234e5c396`](https://github.com/dfinity/ic/commit/234e5c396) Execution,Runtime: Update Wasm benchmarks
* author: Maks Arut | [`2411eb905`](https://github.com/dfinity/ic/commit/2411eb905) Execution,Runtime: rename iDKG key to threshold key
* author: Venk Seka | [`5dc3afeb5`](https://github.com/dfinity/ic/commit/5dc3afeb5) Message Routing,Networking,Runtime(fuzzing): fix clippy warnings for fuzzers
* ~~author: Andr Popo | [`91ceadc58`](https://github.com/dfinity/ic/commit/91ceadc58) Message Routing,NNS(nervous_system): Principals proto typo fix: 7 -> 1 (#375) [AUTO-EXCLUDED]~~
* ~~author: push      | [`f906cf8da`](https://github.com/dfinity/ic/commit/f906cf8da) Owners(github-sync): PR#248 / feat(crypto): add new signature verification package initially supporting canister signatures [AUTO-EXCLUDED]~~
* author: Tim  Gret | [`0775cd819`](https://github.com/dfinity/ic/commit/0775cd819) Networking: abort artifact download externally if peer set is empty
* author: Dani Shar | [`b2268cbaa`](https://github.com/dfinity/ic/commit/b2268cbaa) Networking(ingress-watcher): Add metric to track capacity of the channel from execeution
* ~~author: max-      | [`d732d9d6d`](https://github.com/dfinity/ic/commit/d732d9d6d) NNS: Add api <--> internal type conversions (#393) [AUTO-EXCLUDED]~~
* author: r-bi      | [`eb775492d`](https://github.com/dfinity/ic/commit/eb775492d) Node: firewall counter exporter (#343)
* author: Andr Batt | [`3aae377ca`](https://github.com/dfinity/ic/commit/3aae377ca) Node: Log HostOS config partition (config.ini and deployment.json)
* author: DFIN GitL | [`233657b46`](https://github.com/dfinity/ic/commit/233657b46) Node: Update container base images refs [2024-07-12-0623]
* author: mras      | [`3ba594f48`](https://github.com/dfinity/ic/commit/3ba594f48) pocket-ic: collection of preparatory steps for canister HTTP outcalls in PocketIC and unrelated fixes (#352)
* author: Ulan Dege | [`45aefaf9f`](https://github.com/dfinity/ic/commit/45aefaf9f) Runtime: Derive ParitalEq for all sandbox IPC types (#374)
## Refactoring:
* author: Rost Rume | [`e21c3e74e`](https://github.com/dfinity/ic/commit/e21c3e74e) Consensus,Networking: move the PriorityFn under interfaces and rename the PrioriyFnAndFilterProducer to PriorityFnFactory
* author: Fran Prei | [`1413afe92`](https://github.com/dfinity/ic/commit/1413afe92) crypto-team(crypto): replace ed25519-consensus with ic-crypto-ed25519 in prod (#347)
* ~~author: Andr Popo | [`7d3245ce7`](https://github.com/dfinity/ic/commit/7d3245ce7) NNS(nervous_system): Add fields with better names to NeuronsFundNeuron [AUTO-EXCLUDED]~~
## Tests:
* author: Jack Lloy | [`72e6f39b0`](https://github.com/dfinity/ic/commit/72e6f39b0) crypto-team(crypto): Re-enable NIDKG cheating dealer solving test
* author: Drag Djur | [`de3425fa6`](https://github.com/dfinity/ic/commit/de3425fa6) Execution,IDX,Runtime: make system api test to be state machine test (#377)
* author: Maks Arut | [`c12b4b26d`](https://github.com/dfinity/ic/commit/c12b4b26d) Execution,Runtime: support signing disabled iDKG keys in state_machine_tests
* ~~author: Math Björ | [`364fe4f38`](https://github.com/dfinity/ic/commit/364fe4f38) Financial Integrations: test(icp_ledger):, Get and query all blocks from ledger and archives and fix test_archive_indexing (#398) [AUTO-EXCLUDED]~~
* author: Ognj Mari | [`38c7a5098`](https://github.com/dfinity/ic/commit/38c7a5098) Message Routing,IDX: check canister queue upgrade/downgrade compatibility against published version
* ~~author: Stef Neam | [`0a9901ae4`](https://github.com/dfinity/ic/commit/0a9901ae4) IDX,NNS: remove old hyper from system tests [AUTO-EXCLUDED]~~
* author: Ognj Mari | [`3017e2e4a`](https://github.com/dfinity/ic/commit/3017e2e4a) IDX,NNS,Runtime: move some Bazel rules out of the system test defs
* author: Stef Neam | [`a91bae41e`](https://github.com/dfinity/ic/commit/a91bae41e) Networking: decompress bitcoin data inside tests
* ~~author: dani      | [`2d2f3b550`](https://github.com/dfinity/ic/commit/2d2f3b550) NNS(sns): SNS upgrade-related tests were flaking out. (#391) [AUTO-EXCLUDED]~~
* author: Venk Seka | [`34ff2857a`](https://github.com/dfinity/ic/commit/34ff2857a) Runtime(fuzzing): create new test library `wasm_fuzzers`
## Documentation:
* ~~author: Andr Popo | [`16dc659a0`](https://github.com/dfinity/ic/commit/16dc659a0) NNS(sns): Typo fix ManageVotingPermissions → ManageVotingPermission [AUTO-EXCLUDED]~~
## Other changes:
* ~~author: Dani Wong | [`15beeb6a9`](https://github.com/dfinity/ic/commit/15beeb6a9) NNS: Add and use workspace version of prometheus-parse. [AUTO-EXCLUDED]~~
"""
    )
