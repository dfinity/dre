from commit_annotation import COMMIT_BELONGS, LocalCommitChangeDeterminator
from const import GUESTOS, HOSTOS
from git_repo import GitRepo
from release_notes_composer import (
    Change,
    OrdinaryReleaseNotesRequest,
    SecurityReleaseNotesRequest,
    get_change_description_for_commit,
    prepare_release_notes,
)
from tests.fixtures import ic_repo as ic_repo


def test_get_change_description_for_commit(ic_repo: GitRepo) -> None:
    determinator = LocalCommitChangeDeterminator(ic_repo)

    def testme(commit_hash: str) -> Change:
        belongs = determinator.commit_changes_artifact(commit_hash, GUESTOS)
        return get_change_description_for_commit(
            commit_hash=commit_hash,
            ic_repo=ic_repo,
            belongs=belongs in [COMMIT_BELONGS],
        )

    assert testme(commit_hash="00dc67f8d") == Change(
        commit="00dc67f8d",
        teams=[
            "crypto-team",
        ],
        type="refactor",
        scope="",
        message="Use ic_cdk::api::time for ingress message validator crate ([#802](https://github.com/dfinity/ic/pull/802))",
        commiter="Dimi Sarl",
        exclusion_reason=None,
        belongs_to_this_release=False,
    )
    # bumping dependencies
    assert testme(commit_hash="2d0835bba") == Change(
        commit="2d0835bba",
        teams=[
            "ic-owners-owners",
        ],
        type="chore",
        scope="crypto",
        message="bump ic_bls12_381 to 0.10.0 ([#770](https://github.com/dfinity/ic/pull/770))",
        commiter="Olek Tkac",
        exclusion_reason=None,
        belongs_to_this_release=True,
    )
    # .github change
    assert testme(commit_hash="94fd38099") == Change(
        commit="94fd38099",
        teams=[
            "ic-owners-owners",
        ],
        type="chore",
        scope="IDX",
        message="fix workflow syntax ([#824](https://github.com/dfinity/ic/pull/824))",
        commiter="Carl Gund",
        exclusion_reason=None,
        belongs_to_this_release=False,
    )
    # replica change
    assert testme(commit_hash="951e895c7") == Change(
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
        belongs_to_this_release=True,
    )
    # modifies Cargo.lock but not in a meaningful way
    assert testme(commit_hash="5a250cb34") == Change(
        commit="5a250cb34",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="ic-admin",
        message="Support sending update_canister_settings proposals through ic-admin ([#789](https://github.com/dfinity/ic/pull/789))",
        commiter="jaso     ",
        exclusion_reason=None,
        belongs_to_this_release=False,
    )
    # modifies ic-admin
    assert testme(commit_hash="d436a526d") == Change(
        commit="d436a526d",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="ic-admin",
        message="Print hashes rather than entire blobs when submitting InstallCode proposals ([#1093](https://github.com/dfinity/ic/pull/1093))",
        commiter="jaso     ",
        exclusion_reason="Changed files are excluded by file path filter",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="92e0f4a55") == Change(
        commit="92e0f4a55",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="nns",
        message="Store `wasm_metadata` in SNS-W's stable memory (attempt #2) ([#977](https://github.com/dfinity/ic/pull/977))",
        commiter="Arsh Ter-",
        exclusion_reason="Scope of the change (nns) is not related to the artifact",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="0aa15a5be") == Change(
        commit="0aa15a5be",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="nns",
        message="Automatically set SNS Governance, Ledger, Index, Archive canisters memory limits once ([#1004](https://github.com/dfinity/ic/pull/1004))",
        commiter="Andr Popo",
        exclusion_reason="Changed files are excluded by file path filter",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="974f22dc1") == Change(
        commit="974f22dc1",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="sns",
        message="Expose the wasm_memory_limit in sns_canisters_summary's settings ([#1054](https://github.com/dfinity/ic/pull/1054))",
        commiter="Andr Popo",
        exclusion_reason="Changed files are excluded by file path filter",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="05b02520f") == Change(
        commit="05b02520f",
        teams=[
            "ic-interface-owners",
        ],
        type="feat",
        scope="sns",
        message="Reject new participants if the maximum number of required SNS neurons has been reached ([#924](https://github.com/dfinity/ic/pull/924))",
        commiter="Arsh Ter-",
        exclusion_reason="Scope of the change (sns) is not related to the artifact",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="57293157d") == Change(
        commit="57293157d",
        teams=[
            "ic-interface-owners",
        ],
        type="chore",
        scope="sns",
        message="Remove migration code for setting SNS memory limits ([#1159](https://github.com/dfinity/ic/pull/1159))",
        commiter="Andr Popo",
        exclusion_reason="Changed files are excluded by file path filter",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="f4242cbcf") == Change(
        commit="f4242cbcf",
        teams=[
            "ic-interface-owners",
        ],
        type="chore",
        scope="",
        message="add decoding quota to http_request in NNS root canister ([#1031](https://github.com/dfinity/ic/pull/1031))",
        commiter="mras     ",
        exclusion_reason="Changed files are excluded by file path filter",
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="a63138ab5") == Change(
        commit="a63138ab5",
        teams=[
            "execution",
            "ic-interface-owners",
            "ic-message-routing-owners",
        ],
        type="feat",
        scope="",
        message="Check `SystemState` invariants on checkpoint loading ([#1165](https://github.com/dfinity/ic/pull/1165))",
        commiter="Alin Sinp",
        exclusion_reason=None,
        belongs_to_this_release=True,
    )
    assert testme(commit_hash="ee64a50") == Change(
        commit="ee64a50",
        teams=["team-dsm"],
        type="chore",
        scope="",
        message="Extend log message for invalid stream slices during block making ([#7658](https://github.com/dfinity/ic/pull/7658))",
        commiter="Davi Derl",
        exclusion_reason=None,
        belongs_to_this_release=True,
    )


def test_guestos_release_notes(ic_repo: GitRepo) -> None:
    belongs = LocalCommitChangeDeterminator(ic_repo)
    assert (
        prepare_release_notes(
            OrdinaryReleaseNotesRequest(
                "release-2024-08-02_01-30-base",
                "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d",
                "release-2024-07-25_21-03-base",
                "2c0b76cfc7e596d5c4304cff5222a2619294c8c1",
                GUESTOS,
            ),
            ic_repo,
            belongs,
        )
        == """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>



# Release Notes for [release-2024-08-02_01-30-base](https://github.com/dfinity/ic/tree/release-2024-08-02_01-30-base) (`3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d`)
This release is based on changes since [release-2024-07-25_21-03-base](https://dashboard.internetcomputer.org/release/2c0b76cfc7e596d5c4304cff5222a2619294c8c1) (`2c0b76cfc7e596d5c4304cff5222a2619294c8c1`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-07-25_21-03-base...release-2024-08-02_01-30-base).
## Features:
* author: Adri Alic | [`5e319b9de`](https://github.com/dfinity/ic/commit/5e319b9de) Consensus: Change definition of better to exclude disqualified block makers ([#673](https://github.com/dfinity/ic/pull/673))
* author: Alex      | [`f0093242d`](https://github.com/dfinity/ic/commit/f0093242d) Execution,Runtime: Enforce taking a canister snapshot only when canister is not empty ([#452](https://github.com/dfinity/ic/pull/452))
* author: Dimi Sarl | [`96035ca4c`](https://github.com/dfinity/ic/commit/96035ca4c) Execution,Runtime: Reduce DTS slice limit for regular messages on system subnets ([#621](https://github.com/dfinity/ic/pull/621))
* author: Alex Uta  | [`736beea98`](https://github.com/dfinity/ic/commit/736beea98) Message Routing,Runtime: Enable transparent huge pages for the page allocator ([#665](https://github.com/dfinity/ic/pull/665))
* ~~author: dani      | [`a89a2e17c`](https://github.com/dfinity/ic/commit/a89a2e17c) NNS: Metrics for public neurons. ([#685](https://github.com/dfinity/ic/pull/685)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: dani      | [`448c85ccc`](https://github.com/dfinity/ic/commit/448c85ccc) NNS: Added include_public_neurons_in_full_neurons to ListNeurons. ([#589](https://github.com/dfinity/ic/pull/589)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: jaso      | [`2b109fb9b`](https://github.com/dfinity/ic/commit/2b109fb9b) NNS: Define update_canister_settings proposal type without execution ([#529](https://github.com/dfinity/ic/pull/529)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Bugfixes:
* ~~author: r-bi      | [`d5a950484`](https://github.com/dfinity/ic/commit/d5a950484) Boundary Nodes(ic-boundary): switch logging setup from eager to lazy eval ([#658](https://github.com/dfinity/ic/pull/658)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* author: Adri Alic | [`2bdfdc54c`](https://github.com/dfinity/ic/commit/2bdfdc54c) Consensus: Use correct signer id in make_next_block_with_rank ([#644](https://github.com/dfinity/ic/pull/644))
* author: Dimi Sarl | [`9fc5fc83f`](https://github.com/dfinity/ic/commit/9fc5fc83f) Interface: Update computation of effective canister id for FetchCanisterLogs ([#540](https://github.com/dfinity/ic/pull/540))
* ~~author: Andr Popo | [`395c0e49a`](https://github.com/dfinity/ic/commit/395c0e49a) NNS(sns): Enforce a minimum on the maximum number of permissioned principals an SNS neuron is allowed to have ([#649](https://github.com/dfinity/ic/pull/649)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* author: Rost Rume | [`fd7fc6ebe`](https://github.com/dfinity/ic/commit/fd7fc6ebe) Node: fix our release rules ([#630](https://github.com/dfinity/ic/pull/630))
## Chores:
* author: kpop      | [`204542c15`](https://github.com/dfinity/ic/commit/204542c15) Consensus: change the associated `Error` type of `TryFrom<pb>` from `String` to `ProxyDecodeError` for some consensus types ([#695](https://github.com/dfinity/ic/pull/695))
* author: Dani Shar | [`b4be567dc`](https://github.com/dfinity/ic/commit/b4be567dc) Consensus,Crypto: Bump rust version to 1.80 ([#642](https://github.com/dfinity/ic/pull/642))
* author: Leo  Eich | [`668fbe08f`](https://github.com/dfinity/ic/commit/668fbe08f) Consensus,Execution,Runtime: Rename ECDSA metrics ([#535](https://github.com/dfinity/ic/pull/535))
* author: Rost Rume | [`ec01b3735`](https://github.com/dfinity/ic/commit/ec01b3735) Consensus,Interface: add tools-pkg ([#584](https://github.com/dfinity/ic/pull/584))
* author: Drag Djur | [`4bebd6f6a`](https://github.com/dfinity/ic/commit/4bebd6f6a) Execution,Runtime: Add Wasm memory threshold field to canister settings ([#475](https://github.com/dfinity/ic/pull/475))
* author: Dani Shar | [`219655bf7`](https://github.com/dfinity/ic/commit/219655bf7) Execution,Runtime: Update `agent-rs` dependency version to 0.37.1 ([#560](https://github.com/dfinity/ic/pull/560))
* author: Dimi Sarl | [`0527e6f50`](https://github.com/dfinity/ic/commit/0527e6f50) Message Routing: Use a single sentence for error messages in IngressInductionError ([#648](https://github.com/dfinity/ic/pull/648))
* author: mras      | [`dbfbeceea`](https://github.com/dfinity/ic/commit/dbfbeceea) Networking: bump jemallocator v0.3 to tikv-jemallocator v0.5 ([#654](https://github.com/dfinity/ic/pull/654))
* ~~author: Andr Popo | [`9bc6e18ac`](https://github.com/dfinity/ic/commit/9bc6e18ac) NNS(neurons_fund): Populate hotkeys when necessary in the NNS Governance → Swap → SNS Governance dataflow ([#688](https://github.com/dfinity/ic/pull/688)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* author: sa-g      | [`c77043f06`](https://github.com/dfinity/ic/commit/c77043f06) Node: Update Base Image Refs [2024-08-01-0150] ([#712](https://github.com/dfinity/ic/pull/712))
* author: sa-g      | [`2c8adf74b`](https://github.com/dfinity/ic/commit/2c8adf74b) Node: Update Base Image Refs [2024-07-31-0139] ([#690](https://github.com/dfinity/ic/pull/690))
* author: Rost Rume | [`173d06185`](https://github.com/dfinity/ic/commit/173d06185) Node: build and strip IC-OS tools iff we build the VMs ([#609](https://github.com/dfinity/ic/pull/609))
* author: Maci Kot  | [`f6a88d1a5`](https://github.com/dfinity/ic/commit/f6a88d1a5) Runtime: Saturate function index in system api calls ([#641](https://github.com/dfinity/ic/pull/641))
## Refactoring:
* author: kpop      | [`962bb3848`](https://github.com/dfinity/ic/commit/962bb3848) Consensus: clean up the `dkg::payload_validator` code a bit and increase the test coverage ([#661](https://github.com/dfinity/ic/pull/661))
* author: Fran Prei | [`9ff9f96b0`](https://github.com/dfinity/ic/commit/9ff9f96b0) Crypto: remove CspTlsHandshakeSignerProvider ([#627](https://github.com/dfinity/ic/pull/627))
* author: Fran Prei | [`1909c13a8`](https://github.com/dfinity/ic/commit/1909c13a8) Crypto: remove CspPublicKeyStore ([#625](https://github.com/dfinity/ic/pull/625))
* author: Dimi Sarl | [`50857b09e`](https://github.com/dfinity/ic/commit/50857b09e) Message Routing: Move IngressInductionError outside of replicated state ([#618](https://github.com/dfinity/ic/pull/618))
* ~~author: Andr Popo | [`96bc27800`](https://github.com/dfinity/ic/commit/96bc27800) NNS(sns): Add controller and hotkeys information to ClaimSwapNeuronsRequest, and use it in SNS Governance ([#596](https://github.com/dfinity/ic/pull/596)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: Andr Popo | [`1a0c97fe4`](https://github.com/dfinity/ic/commit/1a0c97fe4) NNS(sns): Remove the open method from swap. [override-didc-check] ([#454](https://github.com/dfinity/ic/pull/454)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Tests:
* author: Dimi Sarl | [`0ed8c497c`](https://github.com/dfinity/ic/commit/0ed8c497c) Consensus,Execution: Fix property tests in bitcoin consensus payload builder ([#656](https://github.com/dfinity/ic/pull/656))
## ~~Other changes not modifying GuestOS~~
* ~~author: maci      | [`9397d7264`](https://github.com/dfinity/ic/commit/9397d7264) Financial Integrations,unknown(icrc-ledger-types): bumping version to 0.1.6 in order to release icrc3 and icrc21 types. ([#509](https://github.com/dfinity/ic/pull/509)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`e81fe6b18`](https://github.com/dfinity/ic/commit/e81fe6b18) IDX: trigger qualifier workflow ([#668](https://github.com/dfinity/ic/pull/668)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Maci Kot  | [`2c324f2d0`](https://github.com/dfinity/ic/commit/2c324f2d0) Networking: Enable wasm64 in ic_starter ([#666](https://github.com/dfinity/ic/pull/666)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: jaso      | [`51cbfe127`](https://github.com/dfinity/ic/commit/51cbfe127) NNS: Enable new topics to be followed ([#710](https://github.com/dfinity/ic/pull/710)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`8605103d3`](https://github.com/dfinity/ic/commit/8605103d3) NNS: Store minted node provider rewards in stable storage ([#591](https://github.com/dfinity/ic/pull/591)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Arsh Ter- | [`c9879cb1a`](https://github.com/dfinity/ic/commit/c9879cb1a) NNS(neurons-fund): Picking a finite number of NNS hotkeys for propagating to Neurons' Fund SNS neurons ([#683](https://github.com/dfinity/ic/pull/683)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`3b3ffedc6`](https://github.com/dfinity/ic/commit/3b3ffedc6) NNS: Disallow SetVisibility ManageNeuron proposals. ([#643](https://github.com/dfinity/ic/pull/643)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`a1fc94c52`](https://github.com/dfinity/ic/commit/a1fc94c52) Pocket IC(PocketIC): new endpoint to list HTTP gateways ([#636](https://github.com/dfinity/ic/pull/636)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`3f8baa2f2`](https://github.com/dfinity/ic/commit/3f8baa2f2) Pocket IC(PocketIC): specify IP address of PocketIC server and HTTP gateway ([#634](https://github.com/dfinity/ic/pull/634)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`bbffc1a2d`](https://github.com/dfinity/ic/commit/bbffc1a2d) IDX: make sure system_test_benchmark tests aren't filtered out ([#696](https://github.com/dfinity/ic/pull/696)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`0efbeeb91`](https://github.com/dfinity/ic/commit/0efbeeb91) IDX: only run system_test_benchmark tests when targeted explicitly ([#693](https://github.com/dfinity/ic/pull/693)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Bas  van  | [`69d4e47c9`](https://github.com/dfinity/ic/commit/69d4e47c9) IDX: support multiple system_test_benchmarks in the system-tests-benchmarks-nightly job ([#691](https://github.com/dfinity/ic/pull/691)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`23545710e`](https://github.com/dfinity/ic/commit/23545710e) IDX(ci): Fix team label job to not fail if label was already present ([#626](https://github.com/dfinity/ic/pull/626)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`3d0b3f104`](https://github.com/dfinity/ic/commit/3d0b3f104) NNS: Fixed a bug where known neuron is not seen as public. ([#699](https://github.com/dfinity/ic/pull/699)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`dd2fe6092`](https://github.com/dfinity/ic/commit/dd2fe6092) Pocket IC(PocketIC): block until HTTP handler starts ([#637](https://github.com/dfinity/ic/pull/637)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`597d0289c`](https://github.com/dfinity/ic/commit/597d0289c) Consensus(backup): Check if the disk usage exceeds threshold only after running ic-replay ([#680](https://github.com/dfinity/ic/pull/680)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`98797bd8f`](https://github.com/dfinity/ic/commit/98797bd8f) Consensus: extract more utility functions into `tests/consensus/utils` ([#639](https://github.com/dfinity/ic/pull/639)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`e006612ff`](https://github.com/dfinity/ic/commit/e006612ff) Consensus: Inline more consensus tests ([#632](https://github.com/dfinity/ic/pull/632)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Leo  Eich | [`b486455bd`](https://github.com/dfinity/ic/commit/b486455bd) Consensus: Inline remaining tECDSA tests ([#619](https://github.com/dfinity/ic/pull/619)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Ulan Dege | [`3e9785f87`](https://github.com/dfinity/ic/commit/3e9785f87) Execution,Runtime: Rename fees_and_limits to icp_config ([#638](https://github.com/dfinity/ic/pull/638)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`6f11a00a2`](https://github.com/dfinity/ic/commit/6f11a00a2) IDX: create minimal image ([#682](https://github.com/dfinity/ic/pull/682)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Carl Gund | [`8447d147e`](https://github.com/dfinity/ic/commit/8447d147e) IDX: switch python tests to self-hosted ([#663](https://github.com/dfinity/ic/pull/663)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: dani      | [`29e7b09ed`](https://github.com/dfinity/ic/commit/29e7b09ed) IDX,NNS(nns): Inform bazel about the NNS & SNS WASMs that were released yesterday. ([#684](https://github.com/dfinity/ic/pull/684)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dimi Sarl | [`52dbd189b`](https://github.com/dfinity/ic/commit/52dbd189b) Networking: Enable canister snapshots in ic-starter ([#692](https://github.com/dfinity/ic/pull/692)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`58370eda9`](https://github.com/dfinity/ic/commit/58370eda9) NNS: Remove DTS config for NNS StateMachine tests (using defaults set at system level) ([#650](https://github.com/dfinity/ic/pull/650)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: max-      | [`e13eea93c`](https://github.com/dfinity/ic/commit/e13eea93c) NNS: remove long deprecated unused method ([#557](https://github.com/dfinity/ic/pull/557)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Andr Batt | [`3909a2cfe`](https://github.com/dfinity/ic/commit/3909a2cfe) Node: Update test driver to use zst images ([#703](https://github.com/dfinity/ic/pull/703)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: mras      | [`147889370`](https://github.com/dfinity/ic/commit/147889370) Utopia: optimize Haskell spec_compliance test build time ([#651](https://github.com/dfinity/ic/pull/651)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`8b66241eb`](https://github.com/dfinity/ic/commit/8b66241eb) Consensus: Run only the colocated consensus performance test on nightly ([#694](https://github.com/dfinity/ic/pull/694)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`2242420f8`](https://github.com/dfinity/ic/commit/2242420f8) Consensus: Run `consensus_performance_test` nightly ([#676](https://github.com/dfinity/ic/pull/676)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: kpop      | [`402a3a6f3`](https://github.com/dfinity/ic/commit/402a3a6f3) Consensus: Push consensus performance test results to Elasticsearch ([#646](https://github.com/dfinity/ic/pull/646)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: oggy      | [`32bc2c260`](https://github.com/dfinity/ic/commit/32bc2c260) Message Routing: Use mainnet binaries for the queues compatibility test ([#419](https://github.com/dfinity/ic/pull/419)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dani Shar | [`d8a956a2e`](https://github.com/dfinity/ic/commit/d8a956a2e) IDX(test-driver): Increase `max_concurrent_requests` in system test agent to 10_000 ([#715](https://github.com/dfinity/ic/pull/715)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
* ~~author: Dani Shar | [`c6cde0abe`](https://github.com/dfinity/ic/commit/c6cde0abe) IDX(call-v3): Make agent to use the v3 call endpoint for system tests ([#635](https://github.com/dfinity/ic/pull/635)) [AUTO-EXCLUDED:Not modifying GuestOS]~~
"""
    )

    res = prepare_release_notes(
        SecurityReleaseNotesRequest(
            "release-2024-08-02_01-30-base",
            "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d",
            GUESTOS,
        ),
        ic_repo,
        belongs,
    )
    assert "accordance" in res, f"No security caveat present in {res}"
    assert "Release Notes for" in res, f"No Release Notes headline present in {res}"


def test_hostos_release_notes(ic_repo: GitRepo) -> None:
    belongs = LocalCommitChangeDeterminator(ic_repo)
    assert (
        prepare_release_notes(
            OrdinaryReleaseNotesRequest(
                "release-2025-04-16_11-12-base",
                "c9210f4d299546658760465d7fde93913989f70b",
                "release-2025-04-11_13-20-base",
                "579b8ba3a31341f354f4ddb3d60ac44548a91bc2",
                HOSTOS,
            ),
            ic_repo,
            belongs,
        )
        == """\
# Review checklist

<span style="color: red">Please cross-out your team once you finished the review</span>

- @node-team

# Release Notes for [release-2025-04-16_11-12-base](https://github.com/dfinity/ic/tree/release-2025-04-16_11-12-base) (`c9210f4d299546658760465d7fde93913989f70b`)
This release is based on changes since [release-2025-04-11_13-20-base](https://dashboard.internetcomputer.org/release/579b8ba3a31341f354f4ddb3d60ac44548a91bc2) (`579b8ba3a31341f354f4ddb3d60ac44548a91bc2`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the HostOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2025-04-11_13-20-base...release-2025-04-16_11-12-base).
## Features:
* author: kpop      | [`6b953276b`](https://github.com/dfinity/ic/commit/6b953276b) Consensus: periodically fetch the nns delegation ([#3902](https://github.com/dfinity/ic/pull/3902))
* author: mich      | [`66ffd5231`](https://github.com/dfinity/ic/commit/66ffd5231) Execution: Charge for snapshot data download ([#4787](https://github.com/dfinity/ic/pull/4787))
* author: mich      | [`23abac589`](https://github.com/dfinity/ic/commit/23abac589) Execution: Enable snapshot data download in statemachine tests ([#4729](https://github.com/dfinity/ic/pull/4729))
* ~~author: max-      | [`c00595a6d`](https://github.com/dfinity/ic/commit/c00595a6d) NNS(registry): Node Rewards can target a specific version ([#4828](https://github.com/dfinity/ic/pull/4828)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Bugfixes:
* author: Leon Tan  | [`56b0c90d2`](https://github.com/dfinity/ic/commit/56b0c90d2) Consensus(consnesus): Fix reshare chain key validation ([#4829](https://github.com/dfinity/ic/pull/4829))
* author: mich      | [`7575e49a4`](https://github.com/dfinity/ic/commit/7575e49a4) Execution: Improve constants in wasm chunk store ([#4712](https://github.com/dfinity/ic/pull/4712))
* ~~author: Math Björ | [`5599a9860`](https://github.com/dfinity/ic/commit/5599a9860) Financial Integrations(ICRC_Ledger): Recompute ICRC ledger certified data in post upgrade ([#4796](https://github.com/dfinity/ic/pull/4796)) [AUTO-EXCLUDED:The change is not owned by any replica or HostOS team]~~
* author: Shuo Wang | [`79f0a7d1f`](https://github.com/dfinity/ic/commit/79f0a7d1f) Message Routing: switch to checkpoint for wasm binaries in canister snapshots ([#4777](https://github.com/dfinity/ic/pull/4777))
* author: Bas  van  | [`c9210f4d2`](https://github.com/dfinity/ic/commit/c9210f4d2) Node: revert "chore: unifying downloading logic ([#4805](https://github.com/dfinity/ic/pull/4805))" ([#4836](https://github.com/dfinity/ic/pull/4836))
* ~~author: Igor Novg | [`e564b0380`](https://github.com/dfinity/ic/commit/e564b0380) Node: api bn: update `ic-gateway`, increase h2 streams, lower shedding threshold ([#4818](https://github.com/dfinity/ic/pull/4818)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Chores:
* author: Andr Batt | [`b60e4861d`](https://github.com/dfinity/ic/commit/b60e4861d) Consensus(node): Improve orchestrator node_operator_private_key.pem logging ([#4753](https://github.com/dfinity/ic/pull/4753))
* author: kpop      | [`6876dcac8`](https://github.com/dfinity/ic/commit/6876dcac8) Consensus(ic-replay): add more logs to `ic-replay` ([#4685](https://github.com/dfinity/ic/pull/4685))
* author: Adam Brat | [`d6c72756c`](https://github.com/dfinity/ic/commit/d6c72756c) Execution: Remove old sandbox rpc calls ([#4728](https://github.com/dfinity/ic/pull/4728))
* author: Andr Bere | [`bd371e73a`](https://github.com/dfinity/ic/commit/bd371e73a) Execution: EXC: Fix flaky monitor thread test ([#4789](https://github.com/dfinity/ic/pull/4789))
* author: Shuo Wang | [`5c0d15487`](https://github.com/dfinity/ic/commit/5c0d15487) Message Routing: Deserialize wasm with hash always present ([#4734](https://github.com/dfinity/ic/pull/4734))
* author: Niko Milo | [`943d3bf19`](https://github.com/dfinity/ic/commit/943d3bf19) Node: unifying downloading logic ([#4805](https://github.com/dfinity/ic/pull/4805))
* author: pr-c      | [`896a78fbe`](https://github.com/dfinity/ic/commit/896a78fbe) Node: Update Base Image Refs [2025-04-15-0151] ([#4814](https://github.com/dfinity/ic/pull/4814))
* author: r-bi      | [`f9a54926d`](https://github.com/dfinity/ic/commit/f9a54926d) Node: export hostos config as metric ([#4785](https://github.com/dfinity/ic/pull/4785))
## Refactoring:
* ~~author: jaso      | [`af2c159bf`](https://github.com/dfinity/ic/commit/af2c159bf) NNS: Initialize NNS Governance with candid ([#4797](https://github.com/dfinity/ic/pull/4797)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## Tests:
* ~~author: max-      | [`8bb84553e`](https://github.com/dfinity/ic/commit/8bb84553e) NNS(node_rewards): Create a test to prove same results as registry ([#4754](https://github.com/dfinity/ic/pull/4754)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
* ~~author: Arsh Ter- | [`d412bc7c7`](https://github.com/dfinity/ic/commit/d412bc7c7) NNS(sns): Faster set-following tests ([#4772](https://github.com/dfinity/ic/pull/4772)) [AUTO-EXCLUDED:Changed files are excluded by file path filter]~~
## ~~Other changes not modifying HostOS~~
* ~~author: Dani Wong | [`ecee8457c`](https://github.com/dfinity/ic/commit/ecee8457c) NNS(registry): Library for chunkifying whale registry mutations. ([#4761](https://github.com/dfinity/ic/pull/4761)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: jaso      | [`13b6e2630`](https://github.com/dfinity/ic/commit/13b6e2630) NNS: Add a timer task to perform voting power snapshots ([#4405](https://github.com/dfinity/ic/pull/4405)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: jaso      | [`e41b1f0c4`](https://github.com/dfinity/ic/commit/e41b1f0c4) NNS: Define NeuronAsyncLock to be compatible with safer access pattern to global state ([#4774](https://github.com/dfinity/ic/pull/4774)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: jaso      | [`6aca5540e`](https://github.com/dfinity/ic/commit/6aca5540e) NNS: Add an index for maturity disbursement based on finalization timestamp ([#4770](https://github.com/dfinity/ic/pull/4770)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: greg      | [`34404b5a8`](https://github.com/dfinity/ic/commit/34404b5a8) Cross Chain: re-enable `ic_xc_cketh_test` ([#4780](https://github.com/dfinity/ic/pull/4780)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: stie      | [`51f052788`](https://github.com/dfinity/ic/commit/51f052788) Message Routing: Make the Heartbeat Counter on the Random Traffic Canister increment only for substantial Heartbeats. ([#4807](https://github.com/dfinity/ic/pull/4807)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: jaso      | [`1c46b8a2c`](https://github.com/dfinity/ic/commit/1c46b8a2c) NNS: Turn off disburse maturity which was incorrectly turned on ([#4827](https://github.com/dfinity/ic/pull/4827)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Paul Liu  | [`39c02b84c`](https://github.com/dfinity/ic/commit/39c02b84c) Cross Chain(ckbtc): Upgrade the btc checker ([#4709](https://github.com/dfinity/ic/pull/4709)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: venk      | [`ce266e1df`](https://github.com/dfinity/ic/commit/ce266e1df) Execution(fuzzing): Switch to ExecutionTest framework from StateMachine ([#4786](https://github.com/dfinity/ic/pull/4786)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Dimi Sarl | [`7c0b90e5a`](https://github.com/dfinity/ic/commit/7c0b90e5a) Execution,Interface: Add a Contributing.md file in rs/embedders ([#4677](https://github.com/dfinity/ic/pull/4677)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Nico Matt | [`bc4751117`](https://github.com/dfinity/ic/commit/bc4751117) IDX: clean up execlogs workflows ([#4791](https://github.com/dfinity/ic/pull/4791)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Nico Matt | [`800d9a1a3`](https://github.com/dfinity/ic/commit/800d9a1a3) IDX: use execution log for determinism checks ([#4771](https://github.com/dfinity/ic/pull/4771)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Dani Wong | [`28ef5ff67`](https://github.com/dfinity/ic/commit/28ef5ff67) NNS: Delete flags related to periodic confirmation of following. ([#3782](https://github.com/dfinity/ic/pull/3782)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: max-      | [`971eecc54`](https://github.com/dfinity/ic/commit/971eecc54) NNS: update changelogs ([#4793](https://github.com/dfinity/ic/pull/4793)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: pr-c      | [`19523bdab`](https://github.com/dfinity/ic/commit/19523bdab) unknown: Update Mainnet IC revisions canisters file ([#4809](https://github.com/dfinity/ic/pull/4809)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: pr-c      | [`44558846e`](https://github.com/dfinity/ic/commit/44558846e) unknown: Update Mainnet IC revisions canisters file ([#4808](https://github.com/dfinity/ic/pull/4808)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: pr-c      | [`512cf412f`](https://github.com/dfinity/ic/commit/512cf412f) unknown: Update Mainnet IC revisions file ([#4806](https://github.com/dfinity/ic/pull/4806)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: pr-c      | [`d95941df2`](https://github.com/dfinity/ic/commit/d95941df2) unknown: Update Mainnet IC revisions file ([#4802](https://github.com/dfinity/ic/pull/4802)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Paul Liu  | [`c90a65062`](https://github.com/dfinity/ic/commit/c90a65062) Cross Chain(ckbtc): Clean up types used by ckbtc minter ([#4757](https://github.com/dfinity/ic/pull/4757)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Leon Tan  | [`593392e05`](https://github.com/dfinity/ic/commit/593392e05) Consensus: Increase number of retries of get signature in system tests ([#4835](https://github.com/dfinity/ic/pull/4835)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Leo  Eich | [`3e3a91cb3`](https://github.com/dfinity/ic/commit/3e3a91cb3) Consensus: Increase subnet size to 4 nodes in recovery tests ([#4830](https://github.com/dfinity/ic/pull/4830)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Bas  van  | [`f46f61285`](https://github.com/dfinity/ic/commit/f46f61285) Execution(IDX): move //rs/execution_environment:execution_environment_misc_integration_tests/dts to its own target ([#4711](https://github.com/dfinity/ic/pull/4711)) [AUTO-EXCLUDED:Not modifying HostOS]~~
* ~~author: Math Björ | [`02eb45caf`](https://github.com/dfinity/ic/commit/02eb45caf) Financial Integrations(ICRC_Ledger): Remove migration-related checks in ICRC ledger suite golden state test ([#4782](https://github.com/dfinity/ic/pull/4782)) [AUTO-EXCLUDED:Not modifying HostOS]~~
"""
    )

    res = prepare_release_notes(
        SecurityReleaseNotesRequest(
            "release-2024-08-02_01-30-base",
            "3d0b3f10417fc6708e8b5d844a0bac5e86f3e17d",
            GUESTOS,
        ),
        ic_repo,
        belongs,
    )
    assert "accordance" in res, f"No security caveat present in {res}"
    assert "Release Notes for" in res, f"No Release Notes headline present in {res}"
