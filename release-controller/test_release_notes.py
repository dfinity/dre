import pathlib
import tempfile
from release_notes import prepare_release_notes, get_change_description_for_commit, Change
from git_repo import GitRepo
import pytest


def test_get_change_description_for_commit():
    with tempfile.TemporaryDirectory() as repo_cache_dir:
        ic_repo = GitRepo(
            "https://github.com/dfinity/ic.git", main_branch="master", repo_cache_dir=pathlib.Path(repo_cache_dir)
        )
        # Not modifying GuestOS
        assert get_change_description_for_commit(commit_hash="00dc67f8d", ic_repo=ic_repo) == Change(
            commit="00dc67f8d",
            teams=[
                "crypto-team",
                "ic-interface-owners",
            ],
            type="refactor",
            scope="",
            message="Use ic_cdk::api::time for ingress message validator crate ([#802](https://github.com/dfinity/ic/pull/802))",
            commiter="Dimi Sarl",
            exclusion_reason=None,
            guestos_change=False,
        )
        # bumping dependencies
        assert get_change_description_for_commit(commit_hash="2d0835bba", ic_repo=ic_repo) == Change(
            commit="2d0835bba",
            teams=[
                "ic-owners-owners",
            ],
            type="chore",
            scope="crypto",
            message="bump ic_bls12_381 to 0.10.0 ([#770](https://github.com/dfinity/ic/pull/770))",
            commiter="Olek Tkac",
            exclusion_reason=None,
            guestos_change=True,
        )
        # .github change
        assert get_change_description_for_commit(commit_hash="94fd38099", ic_repo=ic_repo) == Change(
            commit="94fd38099",
            teams=[
                "ic-owners-owners",
            ],
            type="chore",
            scope="IDX",
            message="fix workflow syntax ([#824](https://github.com/dfinity/ic/pull/824))",
            commiter="Carl Gund",
            exclusion_reason=None,
            guestos_change=False,
        )
        # replica change
        assert get_change_description_for_commit(commit_hash="951e895c7", ic_repo=ic_repo) == Change(
            commit="951e895c7",
            teams=[
                "execution",
                "ic-interface-owners",
            ],
            type="feat",
            scope="",
            message="Handle stable_read/write with Wasm64 heaps and testing infrastructure ([#781](https://github.com/dfinity/ic/pull/781))",
            commiter="Alex Uta ",
            exclusion_reason=None,
            guestos_change=True,
        )
        # modifies Cargo.lock but not in a meaningful way
        assert get_change_description_for_commit(commit_hash="5a250cb34", ic_repo=ic_repo) == Change(
            commit="5a250cb34",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="ic-admin",
            message="Support sending update_canister_settings proposals through ic-admin ([#789](https://github.com/dfinity/ic/pull/789))",
            commiter="jaso     ",
            exclusion_reason=None,
            guestos_change=False,
        )
        # modifies ic-admin
        assert get_change_description_for_commit(commit_hash="d436a526d", ic_repo=ic_repo) == Change(
            commit="d436a526d",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="ic-admin",
            message="Print hashes rather than entire blobs when submitting InstallCode proposals ([#1093](https://github.com/dfinity/ic/pull/1093))",
            commiter="jaso     ",
            exclusion_reason="Changed files are excluded by file path filter",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="92e0f4a55", ic_repo=ic_repo) == Change(
            commit="92e0f4a55",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="nns",
            message="Store `wasm_metadata` in SNS-W's stable memory (attempt #2) ([#977](https://github.com/dfinity/ic/pull/977))",
            commiter="Arsh Ter-",
            exclusion_reason="Scope of the change (nns) is not related to GuestOS",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="0aa15a5be", ic_repo=ic_repo) == Change(
            commit="0aa15a5be",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="nns",
            message="Automatically set SNS Governance, Ledger, Index, Archive canisters memory limits once ([#1004](https://github.com/dfinity/ic/pull/1004))",
            commiter="Andr Popo",
            exclusion_reason="Changed files are excluded by file path filter",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="974f22dc1", ic_repo=ic_repo) == Change(
            commit="974f22dc1",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="sns",
            message="Expose the wasm_memory_limit in sns_canisters_summary's settings ([#1054](https://github.com/dfinity/ic/pull/1054))",
            commiter="Andr Popo",
            exclusion_reason="Changed files are excluded by file path filter",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="05b02520f", ic_repo=ic_repo) == Change(
            commit="05b02520f",
            teams=[
                "ic-interface-owners",
            ],
            type="feat",
            scope="sns",
            message="Reject new participants if the maximum number of required SNS neurons has been reached ([#924](https://github.com/dfinity/ic/pull/924))",
            commiter="Arsh Ter-",
            exclusion_reason="Scope of the change (sns) is not related to GuestOS",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="57293157d", ic_repo=ic_repo) == Change(
            commit="57293157d",
            teams=[
                "ic-interface-owners",
            ],
            type="chore",
            scope="sns",
            message="Remove migration code for setting SNS memory limits ([#1159](https://github.com/dfinity/ic/pull/1159))",
            commiter="Andr Popo",
            exclusion_reason="Changed files are excluded by file path filter",
            guestos_change=True,
        )
        assert get_change_description_for_commit(commit_hash="f4242cbcf", ic_repo=ic_repo) == Change(
            commit="f4242cbcf",
            teams=[
                "ic-interface-owners",
            ],
            type="chore",
            scope="",
            message="add decoding quota to http_request in NNS root canister ([#1031](https://github.com/dfinity/ic/pull/1031))",
            commiter="mras     ",
            exclusion_reason="Changed files are excluded by file path filter",
            guestos_change=True,
        )


def test_release_notes():
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
* ~~author: dani      | [`a89a2e17c`](https://github.com/dfinity/ic/commit/a89a2e17c) Interface(nns): Metrics for public neurons. ([#685](https://github.com/dfinity/ic/pull/685)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: dani      | [`448c85ccc`](https://github.com/dfinity/ic/commit/448c85ccc) Interface(nns): Added include_public_neurons_in_full_neurons to ListNeurons. ([#589](https://github.com/dfinity/ic/pull/589)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: jaso      | [`2b109fb9b`](https://github.com/dfinity/ic/commit/2b109fb9b) Interface(nns): Define update_canister_settings proposal type without execution ([#529](https://github.com/dfinity/ic/pull/529)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Bugfixes:
* author: Adri Alic | [`2bdfdc54c`](https://github.com/dfinity/ic/commit/2bdfdc54c) Consensus,Interface(consensus): Use correct signer id in make_next_block_with_rank ([#644](https://github.com/dfinity/ic/pull/644))
* ~~author: r-bi      | [`d5a950484`](https://github.com/dfinity/ic/commit/d5a950484) Interface(ic-boundary): switch logging setup from eager to lazy eval ([#658](https://github.com/dfinity/ic/pull/658)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: Andr Popo | [`395c0e49a`](https://github.com/dfinity/ic/commit/395c0e49a) Interface(sns): Enforce a minimum on the maximum number of permissioned principals an SNS neuron is allowed to have ([#649](https://github.com/dfinity/ic/pull/649)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* author: Dimi Sarl | [`9fc5fc83f`](https://github.com/dfinity/ic/commit/9fc5fc83f) Interface: Update computation of effective canister id for FetchCanisterLogs ([#540](https://github.com/dfinity/ic/pull/540))
* author: Rost Rume | [`fd7fc6ebe`](https://github.com/dfinity/ic/commit/fd7fc6ebe) Node: fix our release rules ([#630](https://github.com/dfinity/ic/pull/630))
## Chores:
* author: kpop      | [`204542c15`](https://github.com/dfinity/ic/commit/204542c15) Consensus,Interface(consensus): change the associated `Error` type of `TryFrom<pb>` from `String` to `ProxyDecodeError` for some consensus types ([#695](https://github.com/dfinity/ic/pull/695))
* author: Drag Djur | [`4bebd6f6a`](https://github.com/dfinity/ic/commit/4bebd6f6a) Execution,Interface: Add Wasm memory threshold field to canister settings ([#475](https://github.com/dfinity/ic/pull/475))
* author: Andr Popo | [`9bc6e18ac`](https://github.com/dfinity/ic/commit/9bc6e18ac) Interface(neurons_fund): Populate hotkeys when necessary in the NNS Governance → Swap → SNS Governance dataflow ([#688](https://github.com/dfinity/ic/pull/688))
* author: Dani Shar | [`b4be567dc`](https://github.com/dfinity/ic/commit/b4be567dc) Interface: Bump rust version to 1.80 ([#642](https://github.com/dfinity/ic/pull/642))
* author: mras      | [`dbfbeceea`](https://github.com/dfinity/ic/commit/dbfbeceea) Interface: bump jemallocator v0.3 to tikv-jemallocator v0.5 ([#654](https://github.com/dfinity/ic/pull/654))
* author: Leo  Eich | [`668fbe08f`](https://github.com/dfinity/ic/commit/668fbe08f) Interface: Rename ECDSA metrics ([#535](https://github.com/dfinity/ic/pull/535))
* author: Dani Shar | [`219655bf7`](https://github.com/dfinity/ic/commit/219655bf7) Interface: Update `agent-rs` dependency version to 0.37.1 ([#560](https://github.com/dfinity/ic/pull/560))
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
* ~~author: Andr Popo | [`96bc27800`](https://github.com/dfinity/ic/commit/96bc27800) Interface(sns): Add controller and hotkeys information to ClaimSwapNeuronsRequest, and use it in SNS Governance ([#596](https://github.com/dfinity/ic/pull/596)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: Andr Popo | [`1a0c97fe4`](https://github.com/dfinity/ic/commit/1a0c97fe4) Interface(sns): Remove the open method from swap. [override-didc-check] ([#454](https://github.com/dfinity/ic/pull/454)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* author: Dimi Sarl | [`50857b09e`](https://github.com/dfinity/ic/commit/50857b09e) Interface,Message Routing: Move IngressInductionError outside of replicated state ([#618](https://github.com/dfinity/ic/pull/618))
## Tests:
* author: Dimi Sarl | [`0ed8c497c`](https://github.com/dfinity/ic/commit/0ed8c497c) Consensus,Execution,Interface: Fix property tests in bitcoin consensus payload builder ([#656](https://github.com/dfinity/ic/pull/656))
## ~~Other changes not modifying GuestOS~~
* ~~author: jaso      | [`51cbfe127`](https://github.com/dfinity/ic/commit/51cbfe127) Interface(nns): Enable new topics to be followed ([#710](https://github.com/dfinity/ic/pull/710)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`8605103d3`](https://github.com/dfinity/ic/commit/8605103d3) Interface(nns): Store minted node provider rewards in stable storage ([#591](https://github.com/dfinity/ic/pull/591)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Arsh Ter- | [`c9879cb1a`](https://github.com/dfinity/ic/commit/c9879cb1a) Interface(neurons-fund): Picking a finite number of NNS hotkeys for propagating to Neurons' Fund SNS neurons ([#683](https://github.com/dfinity/ic/pull/683)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`a1fc94c52`](https://github.com/dfinity/ic/commit/a1fc94c52) Interface(PocketIC): new endpoint to list HTTP gateways ([#636](https://github.com/dfinity/ic/pull/636)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`3f8baa2f2`](https://github.com/dfinity/ic/commit/3f8baa2f2) Interface(PocketIC): specify IP address of PocketIC server and HTTP gateway ([#634](https://github.com/dfinity/ic/pull/634)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`3b3ffedc6`](https://github.com/dfinity/ic/commit/3b3ffedc6) Interface(nns): Disallow SetVisibility ManageNeuron proposals. ([#643](https://github.com/dfinity/ic/pull/643)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Maci Kot  | [`2c324f2d0`](https://github.com/dfinity/ic/commit/2c324f2d0) Interface,Networking: Enable wasm64 in ic_starter ([#666](https://github.com/dfinity/ic/pull/666)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`e81fe6b18`](https://github.com/dfinity/ic/commit/e81fe6b18) Owners(IDX): trigger qualifier workflow ([#668](https://github.com/dfinity/ic/pull/668)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: maci      | [`9397d7264`](https://github.com/dfinity/ic/commit/9397d7264) Owners(icrc-ledger-types): bumping version to 0.1.6 in order to release icrc3 and icrc21 types. ([#509](https://github.com/dfinity/ic/pull/509)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`3d0b3f104`](https://github.com/dfinity/ic/commit/3d0b3f104) Interface(nns): Fixed a bug where known neuron is not seen as public. ([#699](https://github.com/dfinity/ic/pull/699)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`dd2fe6092`](https://github.com/dfinity/ic/commit/dd2fe6092) Interface(PocketIC): block until HTTP handler starts ([#637](https://github.com/dfinity/ic/pull/637)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`bbffc1a2d`](https://github.com/dfinity/ic/commit/bbffc1a2d) Owners(IDX): make sure system_test_benchmark tests aren't filtered out ([#696](https://github.com/dfinity/ic/pull/696)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`0efbeeb91`](https://github.com/dfinity/ic/commit/0efbeeb91) Owners(IDX): only run system_test_benchmark tests when targeted explicitly ([#693](https://github.com/dfinity/ic/pull/693)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`69d4e47c9`](https://github.com/dfinity/ic/commit/69d4e47c9) Owners(IDX): support multiple system_test_benchmarks in the system-tests-benchmarks-nightly job ([#691](https://github.com/dfinity/ic/pull/691)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`23545710e`](https://github.com/dfinity/ic/commit/23545710e) Owners(ci): Fix team label job to not fail if label was already present ([#626](https://github.com/dfinity/ic/pull/626)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`597d0289c`](https://github.com/dfinity/ic/commit/597d0289c) Consensus,Interface(backup): Check if the disk usage exceeds threshold only after running ic-replay ([#680](https://github.com/dfinity/ic/pull/680)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`98797bd8f`](https://github.com/dfinity/ic/commit/98797bd8f) Consensus,Interface(consensus): extract more utility functions into `tests/consensus/utils` ([#639](https://github.com/dfinity/ic/pull/639)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`e006612ff`](https://github.com/dfinity/ic/commit/e006612ff) Consensus,Interface(consensus): Inline more consensus tests ([#632](https://github.com/dfinity/ic/pull/632)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Leo  Eich | [`b486455bd`](https://github.com/dfinity/ic/commit/b486455bd) Consensus,Interface: Inline remaining tECDSA tests ([#619](https://github.com/dfinity/ic/pull/619)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Ulan Dege | [`3e9785f87`](https://github.com/dfinity/ic/commit/3e9785f87) Execution,Interface,Runtime: Rename fees_and_limits to icp_config ([#638](https://github.com/dfinity/ic/pull/638)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Andr Batt | [`3909a2cfe`](https://github.com/dfinity/ic/commit/3909a2cfe) Interface: Update test driver to use zst images ([#703](https://github.com/dfinity/ic/pull/703)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`58370eda9`](https://github.com/dfinity/ic/commit/58370eda9) Interface(nns): Remove DTS config for NNS StateMachine tests (using defaults set at system level) ([#650](https://github.com/dfinity/ic/pull/650)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`e13eea93c`](https://github.com/dfinity/ic/commit/e13eea93c) Interface(nns): remove long deprecated unused method ([#557](https://github.com/dfinity/ic/pull/557)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dimi Sarl | [`52dbd189b`](https://github.com/dfinity/ic/commit/52dbd189b) Interface,Networking: Enable canister snapshots in ic-starter ([#692](https://github.com/dfinity/ic/pull/692)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`6f11a00a2`](https://github.com/dfinity/ic/commit/6f11a00a2) Owners(IDX): create minimal image ([#682](https://github.com/dfinity/ic/pull/682)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`29e7b09ed`](https://github.com/dfinity/ic/commit/29e7b09ed) Owners(nns): Inform bazel about the NNS & SNS WASMs that were released yesterday. ([#684](https://github.com/dfinity/ic/pull/684)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`8447d147e`](https://github.com/dfinity/ic/commit/8447d147e) Owners(IDX): switch python tests to self-hosted ([#663](https://github.com/dfinity/ic/pull/663)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`147889370`](https://github.com/dfinity/ic/commit/147889370) Owners: optimize Haskell spec_compliance test build time ([#651](https://github.com/dfinity/ic/pull/651)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`8b66241eb`](https://github.com/dfinity/ic/commit/8b66241eb) Consensus,Interface(consensus): Run only the colocated consensus performance test on nightly ([#694](https://github.com/dfinity/ic/pull/694)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`2242420f8`](https://github.com/dfinity/ic/commit/2242420f8) Consensus,Interface(consensus): Run `consensus_performance_test` nightly ([#676](https://github.com/dfinity/ic/pull/676)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`402a3a6f3`](https://github.com/dfinity/ic/commit/402a3a6f3) Consensus,Interface(consensus): Push consensus performance test results to Elasticsearch ([#646](https://github.com/dfinity/ic/pull/646)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dani Shar | [`d8a956a2e`](https://github.com/dfinity/ic/commit/d8a956a2e) Interface(test-driver): Increase `max_concurrent_requests` in system test agent to 10_000 ([#715](https://github.com/dfinity/ic/pull/715)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dani Shar | [`c6cde0abe`](https://github.com/dfinity/ic/commit/c6cde0abe) Interface(call-v3): Make agent to use the v3 call endpoint for system tests ([#635](https://github.com/dfinity/ic/pull/635)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: oggy      | [`32bc2c260`](https://github.com/dfinity/ic/commit/32bc2c260) Interface,Message Routing: Use mainnet binaries for the queues compatibility test ([#419](https://github.com/dfinity/ic/pull/419)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
"""
    )
