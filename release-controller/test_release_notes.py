from release_notes import prepare_release_notes, get_change_description_for_commit, Change
from git_repo import GitRepo
import pytest


def test_get_change_description_for_commit():
    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")
    # not a guestos change
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


@pytest.mark.skip(reason="to expensive to test this currently")
def test_release_notes():
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
* author: Alex      | [`e7a36d5c8`](https://github.com/dfinity/ic/commit/e7a36d5c8) Execution,Interface,Runtime: Handle canister snapshots during subnet splitting ([#412](https://github.com/dfinity/ic/pull/412))
* author: Dimi Sarl | [`59f22753b`](https://github.com/dfinity/ic/commit/59f22753b) Execution,Interface,Runtime: Print instructions consumed in DTS executions in a more readable form
* author: Dimi Sarl | [`9416ad7d0`](https://github.com/dfinity/ic/commit/9416ad7d0) Interface: Compute effective canister id for canister snapshot requests ([#541](https://github.com/dfinity/ic/pull/541))
* ~~author: Andr Popo | [`fd0eafaf4`](https://github.com/dfinity/ic/commit/fd0eafaf4) Interface(sns): Include hash of upgrade args in UpgradeSnsControlledCanister payload text rendering ([#554](https://github.com/dfinity/ic/pull/554)) [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: dani      | [`871efb5cc`](https://github.com/dfinity/ic/commit/871efb5cc) Interface(nns): Added setting neuron visibility. ([#517](https://github.com/dfinity/ic/pull/517)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: jaso      | [`b3ac41768`](https://github.com/dfinity/ic/commit/b3ac41768) Interface(nns): Support StopOrStartCanister proposal action ([#458](https://github.com/dfinity/ic/pull/458))
* ~~author: dani      | [`3625067d6`](https://github.com/dfinity/ic/commit/3625067d6) Interface(nns): Added visibility field to neurons. ([#451](https://github.com/dfinity/ic/pull/451)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Dani Shar | [`faa3c1ad8`](https://github.com/dfinity/ic/commit/faa3c1ad8) Interface(pocket-ic): Support synchronous call endpoint in pocket-ic. ([#348](https://github.com/dfinity/ic/pull/348))
* ~~author: jaso      | [`b8cd861b9`](https://github.com/dfinity/ic/commit/b8cd861b9) Interface: Add bitcoin and cycles ledger canisters to protocol canisters ([#424](https://github.com/dfinity/ic/pull/424)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Niko Milo | [`215fb78b6`](https://github.com/dfinity/ic/commit/215fb78b6) Interface(farm): extending from config testnet ([#359](https://github.com/dfinity/ic/pull/359))
* author: jaso      | [`922a89e6b`](https://github.com/dfinity/ic/commit/922a89e6b) Interface(nns): Create a new proposal action install_code and support non-root canisters ([#394](https://github.com/dfinity/ic/pull/394))
* ~~author: Igor Novg | [`fde205151`](https://github.com/dfinity/ic/commit/fde205151) Interface: ic-boundary: retry on most calls [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: Jaso (Yel | [`891c74208`](https://github.com/dfinity/ic/commit/891c74208) Interface(nns): Create 2 new topics while not allowing following to be set on them [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: Andr Popo | [`42fb959d5`](https://github.com/dfinity/ic/commit/42fb959d5) Interface(nns): Better field names for API type `NeuronsFundNeuronPortion` [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Chri Stie | [`0f3b81c5f`](https://github.com/dfinity/ic/commit/0f3b81c5f) Interface,Message Routing: Implement handling reject signals from incoming stream slices.
* author: Alex Uta  | [`d267d7f0f`](https://github.com/dfinity/ic/commit/d267d7f0f) Interface,Networking: Revert to the memory allocator ([#515](https://github.com/dfinity/ic/pull/515))
* author: Tim  Gret | [`4c03f768f`](https://github.com/dfinity/ic/commit/4c03f768f) Interface,Networking: publish https outcalls adapter with http enabled for dfx
* author: Eero Kell | [`7d70776f8`](https://github.com/dfinity/ic/commit/7d70776f8) Interface,Node: Pull HostOS upgrade file in chunks
* author: Alex Uta  | [`75c57bc48`](https://github.com/dfinity/ic/commit/75c57bc48) Interface,Runtime: Adjust max number of cached sandboxes
* author: Ulan Dege | [`9f25198cf`](https://github.com/dfinity/ic/commit/9f25198cf) Interface,Runtime: Reland switch to compiler sandbox for compilation
## Bugfixes:
* author: Adri Alic | [`4fd343cae`](https://github.com/dfinity/ic/commit/4fd343cae) Consensus,Interface(consensus): Fix inconsistency when purging validated pool below maximum element ([#598](https://github.com/dfinity/ic/pull/598))
* author: Chri Müll | [`9243f5c75`](https://github.com/dfinity/ic/commit/9243f5c75) Consensus,Interface: ic-replay when DTS is enabled
* author: Jack Lloy | [`72e6f39b0`](https://github.com/dfinity/ic/commit/72e6f39b0) Crypto,Interface(crypto): Re-enable NIDKG cheating dealer solving test
* author: Stef Schn | [`fc5913c1c`](https://github.com/dfinity/ic/commit/fc5913c1c) Execution,Interface,Message Routing: Maintain snapshot_ids correctly ([#360](https://github.com/dfinity/ic/pull/360))
* ~~author: Nico Matt | [`3eb105c27`](https://github.com/dfinity/ic/commit/3eb105c27) Execution,Interface,Runtime(IDX): remove unused aarch64 import ([#507](https://github.com/dfinity/ic/pull/507)) [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: Nico Matt | [`d1d720915`](https://github.com/dfinity/ic/commit/d1d720915) Execution,Interface,Runtime(IDX): disable unused aarch64-darwin code ([#486](https://github.com/dfinity/ic/pull/486)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Ulan Dege | [`7708333b2`](https://github.com/dfinity/ic/commit/7708333b2) Execution,Interface,Runtime: Follow up on the reserved cycles limit fix ([#383](https://github.com/dfinity/ic/pull/383))
* author: Stef      | [`dd0be35cb`](https://github.com/dfinity/ic/commit/dd0be35cb) Interface: fifo tracing layers and connections dashboard ([#576](https://github.com/dfinity/ic/pull/576))
* author: max-      | [`994af8f87`](https://github.com/dfinity/ic/commit/994af8f87) Interface(registry): Optimize get_key_family ([#556](https://github.com/dfinity/ic/pull/556))
* author: Rost Rume | [`65c3775eb`](https://github.com/dfinity/ic/commit/65c3775eb) Interface: use idna for parsing domain names ([#414](https://github.com/dfinity/ic/pull/414))
* author: Luka Skug | [`2ef33c956`](https://github.com/dfinity/ic/commit/2ef33c956) Interface(k8s-testnets): adapt firewall rules for k8s testnets ([#436](https://github.com/dfinity/ic/pull/436))
* author: Niko      | [`33187dbe8`](https://github.com/dfinity/ic/commit/33187dbe8) Interface: add e 8 s to icrc 21 ([#340](https://github.com/dfinity/ic/pull/340))
* author: Stef Schn | [`932506f89`](https://github.com/dfinity/ic/commit/932506f89) Interface,Message Routing: Add total_size to CanisterSnapshotBits ([#479](https://github.com/dfinity/ic/pull/479))
* author: Rost Rume | [`3ee248686`](https://github.com/dfinity/ic/commit/3ee248686) Interface,Networking: use the Shutdown struct instead of explicitly passing the cancellation token for the sender side of the consensus manager
* author: Alex Uta  | [`ff9e2941c`](https://github.com/dfinity/ic/commit/ff9e2941c) Interface,Runtime: Cap Wasm64 heap memory size ([#446](https://github.com/dfinity/ic/pull/446))
* author: Alex Uta  | [`d23960734`](https://github.com/dfinity/ic/commit/d23960734) Interface,Runtime: Fix instrumentation for memory.init and table.init in Wasm 64-bit mode ([#442](https://github.com/dfinity/ic/pull/442))
* author: Ulan Dege | [`4a622c04c`](https://github.com/dfinity/ic/commit/4a622c04c) Interface,Runtime: Free SandboxedExecutionController threads ([#354](https://github.com/dfinity/ic/pull/354))
* author: Andr Bere | [`587c1485b`](https://github.com/dfinity/ic/commit/587c1485b) Interface,Runtime: Revert "feat: Switch to compiler sandbox for compilation"
* author: Rost Rume | [`b239fb792`](https://github.com/dfinity/ic/commit/b239fb792) Owners: upgrade the bytes crate since v1.6.0 was yanked due to a bug
## Performance improvements:
* author: Leo  Eich | [`460693f61`](https://github.com/dfinity/ic/commit/460693f61) Consensus,Interface: Reduce cost of cloning tSchnorr inputs ([#344](https://github.com/dfinity/ic/pull/344))
* author: Jack Lloy | [`fac32ae6f`](https://github.com/dfinity/ic/commit/fac32ae6f) Crypto,Interface(crypto): Reduce the size of randomizers during Ed25519 batch verification ([#413](https://github.com/dfinity/ic/pull/413))
* author: Dimi Sarl | [`390135775`](https://github.com/dfinity/ic/commit/390135775) Execution,Interface: Speed up parsing of optional blob in CanisterHttpRequestArgs ([#478](https://github.com/dfinity/ic/pull/478))
## Chores:
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
* author: Maks Arut | [`2411eb905`](https://github.com/dfinity/ic/commit/2411eb905) Execution,Interface: rename iDKG key to threshold key
* author: Dimi Sarl | [`1ba3b5e0b`](https://github.com/dfinity/ic/commit/1ba3b5e0b) Execution,Interface,Message Routing: Update error message for subnet methods that are not allowed through ingress messages ([#574](https://github.com/dfinity/ic/pull/574))
* author: Dimi Sarl | [`d1206f45a`](https://github.com/dfinity/ic/commit/d1206f45a) Execution,Interface,Runtime: Add logs to capture usages of legacy ICQC feature on system subnets ([#607](https://github.com/dfinity/ic/pull/607))
* author: Dimi Sarl | [`bc2755cff`](https://github.com/dfinity/ic/commit/bc2755cff) Execution,Interface,Runtime(execution): Remove wasm_chunk_store flag ([#542](https://github.com/dfinity/ic/pull/542))
* author: Maks Arut | [`7a8c6c69f`](https://github.com/dfinity/ic/commit/7a8c6c69f) Execution,Interface,Runtime: unify ECDSA and tSchnorr signing requests ([#544](https://github.com/dfinity/ic/pull/544))
* author: Dimi Sarl | [`513b2baec`](https://github.com/dfinity/ic/commit/513b2baec) Execution,Interface,Runtime(management-canister): Remove unimplemented delete_chunks API ([#537](https://github.com/dfinity/ic/pull/537))
* author: Maks Arut | [`e41aefe34`](https://github.com/dfinity/ic/commit/e41aefe34) Execution,Interface,Runtime: remove obsolete canister_logging feature flag ([#505](https://github.com/dfinity/ic/pull/505))
* author: Dimi Sarl | [`005885513`](https://github.com/dfinity/ic/commit/005885513) Execution,Interface,Runtime: Remove deprecated controller field in update settings requests ([#432](https://github.com/dfinity/ic/pull/432))
* ~~author: maci      | [`3ecb66f20`](https://github.com/dfinity/ic/commit/3ecb66f20) Interface(ICP/ICRC-ledger): return value in BalanceStrore.get_balance ([#518](https://github.com/dfinity/ic/pull/518)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Dimi Sarl | [`c4eb29da7`](https://github.com/dfinity/ic/commit/c4eb29da7) Interface: Remove unused instruction limits from subnet record ([#441](https://github.com/dfinity/ic/pull/441))
* author: Maks Arut | [`eec6107fa`](https://github.com/dfinity/ic/commit/eec6107fa) Interface: remove obsolete cost scaling feature flag ([#502](https://github.com/dfinity/ic/pull/502))
* ~~author: maci      | [`14836b59d`](https://github.com/dfinity/ic/commit/14836b59d) Interface(ICP/ICRC-Ledger): refactor approvals library to allow using regular and stable allowance storage ([#382](https://github.com/dfinity/ic/pull/382)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Rost Rume | [`4cc989aa3`](https://github.com/dfinity/ic/commit/4cc989aa3) Interface: upgrade url and uuid and use workspace versions ([#417](https://github.com/dfinity/ic/pull/417))
* ~~author: r-bi      | [`9a3aa19d7`](https://github.com/dfinity/ic/commit/9a3aa19d7) Interface(ic-boundary): removing deprecated CLI option ([#404](https://github.com/dfinity/ic/pull/404)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Rost Rume | [`c52bf40a1`](https://github.com/dfinity/ic/commit/c52bf40a1) Interface: upgrade rustls
* author: Rost Rume | [`5cfaea5ea`](https://github.com/dfinity/ic/commit/5cfaea5ea) Interface: upgrade external crates and use workspace version
* author: Stef Neam | [`0a9901ae4`](https://github.com/dfinity/ic/commit/0a9901ae4) Interface: remove old hyper from system tests
* ~~author: Andr Popo | [`91ceadc58`](https://github.com/dfinity/ic/commit/91ceadc58) Interface,Message Routing(nervous_system): Principals proto typo fix: 7 -> 1 ([#375](https://github.com/dfinity/ic/pull/375)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Dani Shar | [`deafb0a12`](https://github.com/dfinity/ic/commit/deafb0a12) Interface,Networking(http-endpoint): Increase `SETTINGS_MAX_CONCURRENT_STREAMS` to 1000 ([#349](https://github.com/dfinity/ic/pull/349))
* author: Tim  Gret | [`0775cd819`](https://github.com/dfinity/ic/commit/0775cd819) Interface,Networking: abort artifact download externally if peer set is empty
* author: Dani Shar | [`b2268cbaa`](https://github.com/dfinity/ic/commit/b2268cbaa) Interface,Networking(ingress-watcher): Add metric to track capacity of the channel from execeution
* author: Venk Seka | [`5dc3afeb5`](https://github.com/dfinity/ic/commit/5dc3afeb5) Interface,Networking,Runtime(fuzzing): fix clippy warnings for fuzzers
* author: r-bi      | [`eb775492d`](https://github.com/dfinity/ic/commit/eb775492d) Interface,Node: firewall counter exporter ([#343](https://github.com/dfinity/ic/pull/343))
* author: Ulan Dege | [`45aefaf9f`](https://github.com/dfinity/ic/commit/45aefaf9f) Interface,Runtime: Derive ParitalEq for all sandbox IPC types ([#374](https://github.com/dfinity/ic/pull/374))
* author: r-bi      | [`af87b88ac`](https://github.com/dfinity/ic/commit/af87b88ac) Owners: bump response verification and associated crates ([#590](https://github.com/dfinity/ic/pull/590))
* author: Jack Lloy | [`72f9e6d7f`](https://github.com/dfinity/ic/commit/72f9e6d7f) Owners(crypto): Always optimize the curve25519-dalek crate
* author: sa-g      | [`1999421a1`](https://github.com/dfinity/ic/commit/1999421a1) Node: Update Base Image Refs [2024-07-25-0808] ([#601](https://github.com/dfinity/ic/pull/601))
* author: sa-g      | [`c488577bc`](https://github.com/dfinity/ic/commit/c488577bc) Node: Update Base Image Refs [2024-07-20-0145] ([#492](https://github.com/dfinity/ic/pull/492))
* author: sa-g      | [`52b65a8af`](https://github.com/dfinity/ic/commit/52b65a8af) Node: Update Base Image Refs [2024-07-17-0147] ([#397](https://github.com/dfinity/ic/pull/397))
* author: Andr Batt | [`3aae377ca`](https://github.com/dfinity/ic/commit/3aae377ca) Node: Log HostOS config partition (config.ini and deployment.json)
* author: DFIN GitL | [`233657b46`](https://github.com/dfinity/ic/commit/233657b46) Node: Update container base images refs [2024-07-12-0623]
## Refactoring:
* author: Fran Prei | [`5b8fc4237`](https://github.com/dfinity/ic/commit/5b8fc4237) Crypto,Interface(crypto): remove CspPublicAndSecretKeyStoreChecker ([#559](https://github.com/dfinity/ic/pull/559))
* author: Fran Prei | [`63da4b23a`](https://github.com/dfinity/ic/commit/63da4b23a) Crypto,Interface(crypto): unify threshold sign method names ([#321](https://github.com/dfinity/ic/pull/321))
* author: Fran Prei | [`1413afe92`](https://github.com/dfinity/ic/commit/1413afe92) Crypto,Interface(crypto): replace ed25519-consensus with ic-crypto-ed25519 in prod ([#347](https://github.com/dfinity/ic/pull/347))
* ~~author: stie      | [`61870cc77`](https://github.com/dfinity/ic/commit/61870cc77) Execution,Interface,Message Routing: Remove misleading `callback_id` from `register_callback()` test function ([#497](https://github.com/dfinity/ic/pull/497)) [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: max-      | [`2926051d5`](https://github.com/dfinity/ic/commit/2926051d5) Interface(nns): Move governance::init to its own crate to further split type dependencies ([#490](https://github.com/dfinity/ic/pull/490)) [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: Andr Popo | [`a7f5db70e`](https://github.com/dfinity/ic/commit/a7f5db70e) Interface(nervous_system): Add `controller` and `hotkeys` fields to CfParticipant, CfNeuron, and CfInvestment ([#373](https://github.com/dfinity/ic/pull/373)) [AUTO-EXCLUDED:filtered out by package filters]~~
* ~~author: max-      | [`d0a0cc72a`](https://github.com/dfinity/ic/commit/d0a0cc72a) Interface(nns): Use governance_api instead of governance types in entrypoint in governance ([#457](https://github.com/dfinity/ic/pull/457)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Andr Popo | [`8a852bed9`](https://github.com/dfinity/ic/commit/8a852bed9) Interface(nervous_system): Move `Principals` message definition to nervous_system/proto ([#447](https://github.com/dfinity/ic/pull/447))
* ~~author: Andr Popo | [`7d3245ce7`](https://github.com/dfinity/ic/commit/7d3245ce7) Interface(nervous_system): Add fields with better names to NeuronsFundNeuron [AUTO-EXCLUDED:filtered out by package filters]~~
* author: tim  gret | [`f3628917c`](https://github.com/dfinity/ic/commit/f3628917c) Interface,Networking: introduce artifact downloader component ([#403](https://github.com/dfinity/ic/pull/403))
* author: Rost Rume | [`e21c3e74e`](https://github.com/dfinity/ic/commit/e21c3e74e) Interface,Networking: move the PriorityFn under interfaces and rename the PrioriyFnAndFilterProducer to PriorityFnFactory
## Tests:
* ~~author: Ulan Dege | [`e15d65e1c`](https://github.com/dfinity/ic/commit/e15d65e1c) Execution,Interface,Runtime: Add execution smoke tests ([#526](https://github.com/dfinity/ic/pull/526)) [AUTO-EXCLUDED:filtered out by package filters]~~
* author: Maks Arut | [`c12b4b26d`](https://github.com/dfinity/ic/commit/c12b4b26d) Execution,Interface,Runtime: support signing disabled iDKG keys in state_machine_tests
* author: Ulan Dege | [`ba82afe4d`](https://github.com/dfinity/ic/commit/ba82afe4d) Interface,Runtime: Add unit tests for sandbox to replica IPC messages ([#435](https://github.com/dfinity/ic/pull/435))
* author: Ulan Dege | [`9552f0828`](https://github.com/dfinity/ic/commit/9552f0828) Interface,Runtime: Add unit tests for replica to sandbox IPC messages ([#411](https://github.com/dfinity/ic/pull/411))
## Documentation:
* ~~author: Andr Popo | [`16dc659a0`](https://github.com/dfinity/ic/commit/16dc659a0) Interface(sns): Typo fix ManageVotingPermissions → ManageVotingPermission [AUTO-EXCLUDED:filtered out by package filters]~~
## ~~Other changes not modifying GuestOS~~
* ~~author: greg      | [`5519637d2`](https://github.com/dfinity/ic/commit/5519637d2) Interface(ckerc20): NNS proposal to add ckUSDT ([#433](https://github.com/dfinity/ic/pull/433)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`0838df154`](https://github.com/dfinity/ic/commit/0838df154) Interface(ckerc20): Use EVM-RPC canister to call `eth_feeHistory` ([#508](https://github.com/dfinity/ic/pull/508)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko      | [`67e53cc29`](https://github.com/dfinity/ic/commit/67e53cc29) Interface(ICP-Rosetta): add rosetta blocks to block/transaction endpoint ([#524](https://github.com/dfinity/ic/pull/524)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: dani      | [`163f1b0a8`](https://github.com/dfinity/ic/commit/163f1b0a8) Interface(nns): Known neurons are always public. ([#488](https://github.com/dfinity/ic/pull/488)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`476955407`](https://github.com/dfinity/ic/commit/476955407) Interface(PocketIC): canister HTTP outcalls ([#421](https://github.com/dfinity/ic/pull/421)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`1add852d7`](https://github.com/dfinity/ic/commit/1add852d7) Interface(ckerc20): Use EVM-RPC canister to call `eth_getLogs` ([#400](https://github.com/dfinity/ic/pull/400)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Luka Skug | [`ff4fba1a5`](https://github.com/dfinity/ic/commit/ff4fba1a5) Interface(k8s-testnets): playnet certificates ([#437](https://github.com/dfinity/ic/pull/437)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`5237d0cbc`](https://github.com/dfinity/ic/commit/5237d0cbc) Interface(PocketIC): store registry file in state_dir ([#356](https://github.com/dfinity/ic/pull/356)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: jaso      | [`de29a1a55`](https://github.com/dfinity/ic/commit/de29a1a55) Interface(nns): Support upgrading root canister through install_code ([#396](https://github.com/dfinity/ic/pull/396)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`ce468ecac`](https://github.com/dfinity/ic/commit/ce468ecac) Interface(ckerc20): Simplify adding new ckERC20 token (II) ([#365](https://github.com/dfinity/ic/pull/365)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`f38380772`](https://github.com/dfinity/ic/commit/f38380772) Interface(ckerc20): add ckWBTC () ([#364](https://github.com/dfinity/ic/pull/364)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mari Past | [`5ac0b1653`](https://github.com/dfinity/ic/commit/5ac0b1653) Interface: transaction uniqueness in Rosetta Blocks [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko Haim | [`5bba7bd69`](https://github.com/dfinity/ic/commit/5bba7bd69) Interface(ICP-Rosetta): Add query block range [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko Milo | [`27902feb6`](https://github.com/dfinity/ic/commit/27902feb6) Interface(testnets): topology for testnet from config file [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Grég Dema | [`ff90a5234`](https://github.com/dfinity/ic/commit/ff90a5234) Interface(ckerc20): Simplify adding new ckERC20 token [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Wong | [`999054302`](https://github.com/dfinity/ic/commit/999054302) Interface(nns): Exclude the neurons controlled by the Genesis Token canister from metrics about neurons that have a controller that is not self-authenticating. [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mari Past | [`a9d1d1052`](https://github.com/dfinity/ic/commit/a9d1d1052) Interface: support Rosetta Blocks in /blocks in icp rosetta [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mart Rasz | [`1b550f2d0`](https://github.com/dfinity/ic/commit/1b550f2d0) Interface,Networking(PocketIC): non-mainnet features [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`9619db245`](https://github.com/dfinity/ic/commit/9619db245) Owners(IDX): team labels ([#558](https://github.com/dfinity/ic/pull/558)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`b4d49439c`](https://github.com/dfinity/ic/commit/b4d49439c) Owners(IDX): add commit to release channel message ([#613](https://github.com/dfinity/ic/pull/613)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`14ece195b`](https://github.com/dfinity/ic/commit/14ece195b) Owners(IDX): limit namespace runs to 1 ([#536](https://github.com/dfinity/ic/pull/536)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`aa89e8079`](https://github.com/dfinity/ic/commit/aa89e8079) Owners(IDX): Add Apple Silicon builds ([#512](https://github.com/dfinity/ic/pull/512)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`ebeb49ea8`](https://github.com/dfinity/ic/commit/ebeb49ea8) Owners(IDX): wait queue notifications ([#409](https://github.com/dfinity/ic/pull/409)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Jaso (Yel | [`f9c308a73`](https://github.com/dfinity/ic/commit/f9c308a73) Node(release): List commits changing rs/sns/init for NNS Governance and SNS-Wasm [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Luc  Blae | [`1955f41a9`](https://github.com/dfinity/ic/commit/1955f41a9) Execution,Interface(drun): Make drun deterministic again ([#552](https://github.com/dfinity/ic/pull/552)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: tim  gret | [`4c2c49554`](https://github.com/dfinity/ic/commit/4c2c49554) Interface: add missing large testnet runtime dependency ([#468](https://github.com/dfinity/ic/pull/468)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`bb2387f17`](https://github.com/dfinity/ic/commit/bb2387f17) Interface(PocketIC): make CallRequest of type V3 deterministic ([#493](https://github.com/dfinity/ic/pull/493)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko      | [`322c53786`](https://github.com/dfinity/ic/commit/322c53786) Interface(ICP-Rosetta): rosetta block performance issue ([#405](https://github.com/dfinity/ic/pull/405)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`3a31b54c3`](https://github.com/dfinity/ic/commit/3a31b54c3) Interface(IDX): double CPU reservation for //rs/nervous_system/integration_tests:integration_tests_test_tests/sns_ledger_upgrade ([#428](https://github.com/dfinity/ic/pull/428)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: r-bi      | [`99e8b75bb`](https://github.com/dfinity/ic/commit/99e8b75bb) Interface(custom domains): ignore leading and trailing whitespaces when checking domain names ([#361](https://github.com/dfinity/ic/pull/361)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko      | [`18243444a`](https://github.com/dfinity/ic/commit/18243444a) Interface(ICRC-Index): remove comment on removing 0 balance accounts ([#341](https://github.com/dfinity/ic/pull/341)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mart Rasz | [`c7bf31924`](https://github.com/dfinity/ic/commit/c7bf31924) Interface(PocketIC): make sure progress threads stop when deleting PocketIC instance [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Grég Dema | [`f420b4d6e`](https://github.com/dfinity/ic/commit/f420b4d6e) Interface(ckerc20): Stuck ckERC20 withdrawal when fee increases [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`ce2486a3e`](https://github.com/dfinity/ic/commit/ce2486a3e) Interface,Message Routing(IDX): add missing WASM_PATH env vars to the release-testing systests ([#570](https://github.com/dfinity/ic/pull/570)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mart Rasz | [`befc5a404`](https://github.com/dfinity/ic/commit/befc5a404) Interface,Message Routing(PocketIC): resource leak in PocketIC server and bug in PocketIC library [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`dedeaf720`](https://github.com/dfinity/ic/commit/dedeaf720) Owners(IDX): don't invoke gitlab-ci/src/bazel-ci/diff.sh twice ([#623](https://github.com/dfinity/ic/pull/623)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`1866aa854`](https://github.com/dfinity/ic/commit/1866aa854) Owners(IDX): slack alerts syntax error ([#622](https://github.com/dfinity/ic/pull/622)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: venk      | [`e44bf9aa2`](https://github.com/dfinity/ic/commit/e44bf9aa2) Owners(dependency-mgmt): Adapt `ic` root directory checks ([#615](https://github.com/dfinity/ic/pull/615)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`cb69def40`](https://github.com/dfinity/ic/commit/cb69def40) Owners(IDX): add token to workflow ([#614](https://github.com/dfinity/ic/pull/614)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`1bbab0e34`](https://github.com/dfinity/ic/commit/1bbab0e34) Owners(IDX): add correct permissions for dependencies-check ([#605](https://github.com/dfinity/ic/pull/605)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Fran Prei | [`3720193fc`](https://github.com/dfinity/ic/commit/3720193fc) Owners(crypto): make crypto-team own ic-signature-verification package ([#571](https://github.com/dfinity/ic/pull/571)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`70e5f7e61`](https://github.com/dfinity/ic/commit/70e5f7e61) Owners(IDX): syntax error ([#568](https://github.com/dfinity/ic/pull/568)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`a3bb3cb69`](https://github.com/dfinity/ic/commit/a3bb3cb69) Owners(IDX): exclude certain tests from the merge train ([#534](https://github.com/dfinity/ic/pull/534)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`1f5acb4ad`](https://github.com/dfinity/ic/commit/1f5acb4ad) Owners(IDX): inline namespace profile ([#555](https://github.com/dfinity/ic/pull/555)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`ae163b85d`](https://github.com/dfinity/ic/commit/ae163b85d) Owners(IDX): update release testing channel ([#532](https://github.com/dfinity/ic/pull/532)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`24278eb74`](https://github.com/dfinity/ic/commit/24278eb74) Owners(IDX): fix the did_git_test on GitHub ([#480](https://github.com/dfinity/ic/pull/480)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`1925b7064`](https://github.com/dfinity/ic/commit/1925b7064) Owners(IDX): remove undefined CI_COMMIT_REF_NAME in lock-generate.sh ([#495](https://github.com/dfinity/ic/pull/495)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`1b9c124f7`](https://github.com/dfinity/ic/commit/1b9c124f7) Owners(IDX): make lock-generate.sh work on GitHub ([#487](https://github.com/dfinity/ic/pull/487)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`d7097b0ef`](https://github.com/dfinity/ic/commit/d7097b0ef) Owners(IDX): move build filters ([#482](https://github.com/dfinity/ic/pull/482)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`718942d68`](https://github.com/dfinity/ic/commit/718942d68) Owners(IDX): set channel correctly ([#473](https://github.com/dfinity/ic/pull/473)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`d4e510aa9`](https://github.com/dfinity/ic/commit/d4e510aa9) Owners(IDX): codeowners and notifications ([#463](https://github.com/dfinity/ic/pull/463)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`0fb2285ed`](https://github.com/dfinity/ic/commit/0fb2285ed) Owners(IDX): handle name ([#460](https://github.com/dfinity/ic/pull/460)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`81d5f1809`](https://github.com/dfinity/ic/commit/81d5f1809) Owners(IDX): fixing URL syntax ([#430](https://github.com/dfinity/ic/pull/430)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`fe231b385`](https://github.com/dfinity/ic/commit/fe231b385) Owners(IDX): reflect the changed dept-crypto-library team name to crypto-team ([#420](https://github.com/dfinity/ic/pull/420)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: venk      | [`2e269b77f`](https://github.com/dfinity/ic/commit/2e269b77f) Owners(dependency-mgmt): Export `GITHUB_TOKEN` for `dependency-scan-release-cut`  ([#406](https://github.com/dfinity/ic/pull/406)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`4ba2d48f6`](https://github.com/dfinity/ic/commit/4ba2d48f6) Owners(IDX): use correct team names in .github/workflows/team-channels.json ([#401](https://github.com/dfinity/ic/pull/401)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: venk      | [`c2307201b`](https://github.com/dfinity/ic/commit/c2307201b) Owners(dependency-mgmt): Set `fetch-depth` to 256 for `dependencies-check` ([#372](https://github.com/dfinity/ic/pull/372)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`27ef4655c`](https://github.com/dfinity/ic/commit/27ef4655c) Owners(IDX): update pre-commit logic for ic-private ([#370](https://github.com/dfinity/ic/pull/370)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`668b0bf47`](https://github.com/dfinity/ic/commit/668b0bf47) Owners(IDX): add token to lock-generate ([#350](https://github.com/dfinity/ic/pull/350)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`99e513d77`](https://github.com/dfinity/ic/commit/99e513d77) Owners(IDX): update finint team name ([#345](https://github.com/dfinity/ic/pull/345)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`416d937fc`](https://github.com/dfinity/ic/commit/416d937fc) Owners(IDX): use the correct capitalized InfraSec team name in slack-workflow-run ([#334](https://github.com/dfinity/ic/pull/334)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`bd6284524`](https://github.com/dfinity/ic/commit/bd6284524) Owners(IDX): /cache for benchmarks ([#333](https://github.com/dfinity/ic/pull/333)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`548a180f9`](https://github.com/dfinity/ic/commit/548a180f9) Owners(IDX): remove trigger push on master and keep schedule [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: r-bi      | [`47538ff03`](https://github.com/dfinity/ic/commit/47538ff03) Node(testnet): succeed if certs can be fetched from at least one machine ([#448](https://github.com/dfinity/ic/pull/448)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: nabd      | [`fb898b290`](https://github.com/dfinity/ic/commit/fb898b290) Node: fix testonly in icos_deploy bazel rules ([#369](https://github.com/dfinity/ic/pull/369)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Leo  Eich | [`ab43272aa`](https://github.com/dfinity/ic/commit/ab43272aa) Consensus,Interface: Extract a tECDSA system test library and inline some tests ([#608](https://github.com/dfinity/ic/pull/608)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Olek Tkac | [`f0f7659a8`](https://github.com/dfinity/ic/commit/f0f7659a8) Consensus,Interface(consensus): fix uploading of tECDSA benchmark system test results ([#575](https://github.com/dfinity/ic/pull/575)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`311f6a76e`](https://github.com/dfinity/ic/commit/311f6a76e) Consensus,Interface(consensus): inline `node_registration_test` and `ssh_access_to_nodes_test` system tests ([#481](https://github.com/dfinity/ic/pull/481)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`c54b0eb81`](https://github.com/dfinity/ic/commit/c54b0eb81) Consensus,Interface(consensus): move `set_sandbox_env_vars` function to `consensus_system_test_utils` ([#472](https://github.com/dfinity/ic/pull/472)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`53e47573a`](https://github.com/dfinity/ic/commit/53e47573a) Consensus,Interface(consensus): move `ssh_access` to `consensus_system_test_utils` crate ([#471](https://github.com/dfinity/ic/pull/471)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`4f9c5dce3`](https://github.com/dfinity/ic/commit/4f9c5dce3) Consensus,Interface(consensus): Inline `adding_nodes_to_subnet_test` and `node_reassignment_test` system tests ([#466](https://github.com/dfinity/ic/pull/466)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`ea2f05d23`](https://github.com/dfinity/ic/commit/ea2f05d23) Consensus,Interface(consensus): move `rw_message.rs` out of `/rs/tests/src` into `/rs/tests/consensus/utils` ([#378](https://github.com/dfinity/ic/pull/378)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: kpop      | [`10cca1d6f`](https://github.com/dfinity/ic/commit/10cca1d6f) Consensus,Interface(consensus): Deduplicate code in {consensus,tecdsa}_performance_test  ([#346](https://github.com/dfinity/ic/pull/346)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Kami Popi | [`3f5b078b7`](https://github.com/dfinity/ic/commit/3f5b078b7) Consensus,Interface(consensus): inline `consensus_performance` system test [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Leo  Eich | [`b3b624fce`](https://github.com/dfinity/ic/commit/b3b624fce) Consensus,Node: Manually update `testnet/mainnet_revisions.json` ([#573](https://github.com/dfinity/ic/pull/573)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: CI   Auto | [`e6b1f8b40`](https://github.com/dfinity/ic/commit/e6b1f8b40) Consensus,Node: Update Mainnet IC revisions file [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: CI   Auto | [`09bc55871`](https://github.com/dfinity/ic/commit/09bc55871) Consensus,Node: Update Mainnet IC revisions file [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Andr Bere | [`234e5c396`](https://github.com/dfinity/ic/commit/234e5c396) Execution,Interface,Runtime: Update Wasm benchmarks [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`2735cf2e4`](https://github.com/dfinity/ic/commit/2735cf2e4) Interface(IDX): skip system-tests on darwin ([#610](https://github.com/dfinity/ic/pull/610)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`587ffff53`](https://github.com/dfinity/ic/commit/587ffff53) Interface(ckerc20): NNS proposal to upgrade ledger suite orchestrator ([#427](https://github.com/dfinity/ic/pull/427)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`fd136861c`](https://github.com/dfinity/ic/commit/fd136861c) Interface: don't not upload/compress test canisters ([#561](https://github.com/dfinity/ic/pull/561)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Shar | [`087df9b7f`](https://github.com/dfinity/ic/commit/087df9b7f) Interface: change file extension for system test logs ([#546](https://github.com/dfinity/ic/pull/546)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko      | [`cec100d16`](https://github.com/dfinity/ic/commit/cec100d16) Interface(ICRC-Rosetta): add secp key test ([#467](https://github.com/dfinity/ic/pull/467)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`16c2d6877`](https://github.com/dfinity/ic/commit/16c2d6877) Interface(PocketIC): release server v5.0.0 and library v4.0.0 ([#485](https://github.com/dfinity/ic/pull/485)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Shar | [`200fdf578`](https://github.com/dfinity/ic/commit/200fdf578) Interface(test-dashboard): Update message routing dashboards to mirror productions ([#539](https://github.com/dfinity/ic/pull/539)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`14f088b87`](https://github.com/dfinity/ic/commit/14f088b87) Interface(IDX): set wasm paths via env ([#483](https://github.com/dfinity/ic/pull/483)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko      | [`6764190a8`](https://github.com/dfinity/ic/commit/6764190a8) Interface(ICP-Rosetta): enable feature flag rosetta blocks ([#465](https://github.com/dfinity/ic/pull/465)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`a5f5c187c`](https://github.com/dfinity/ic/commit/a5f5c187c) Interface: inline the basic_health_test ([#449](https://github.com/dfinity/ic/pull/449)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: max-      | [`d732d9d6d`](https://github.com/dfinity/ic/commit/d732d9d6d) Interface(nns): Add api <--> internal type conversions ([#393](https://github.com/dfinity/ic/pull/393)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`4b2983084`](https://github.com/dfinity/ic/commit/4b2983084) Interface(PocketIC): refactor progress threads in PocketIC ([#353](https://github.com/dfinity/ic/pull/353)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`3ba594f48`](https://github.com/dfinity/ic/commit/3ba594f48) Interface: collection of preparatory steps for canister HTTP outcalls in PocketIC and unrelated fixes ([#352](https://github.com/dfinity/ic/pull/352)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`073c2bc1f`](https://github.com/dfinity/ic/commit/073c2bc1f) Interface(IDX): make nns_dapp tests bazel-agnostic ([#389](https://github.com/dfinity/ic/pull/389)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: greg      | [`a5831a57c`](https://github.com/dfinity/ic/commit/a5831a57c) Interface(ckerc20): fix dfx.json ([#351](https://github.com/dfinity/ic/pull/351)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Kami Popi | [`96aef6a97`](https://github.com/dfinity/ic/commit/96aef6a97) Interface(consensus): inline `tecdsa_performance_test` [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Ognj Mari | [`3017e2e4a`](https://github.com/dfinity/ic/commit/3017e2e4a) Interface: move some Bazel rules out of the system test defs [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`11bc5648c`](https://github.com/dfinity/ic/commit/11bc5648c) Interface,Networking: publish ic-https-outcalls-adapter-https-only ([#578](https://github.com/dfinity/ic/pull/578)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Stef Neam | [`a91bae41e`](https://github.com/dfinity/ic/commit/a91bae41e) Interface,Networking: decompress bitcoin data inside tests [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`3d1337795`](https://github.com/dfinity/ic/commit/3d1337795) Interface,Node: make the visibility rules consistent ([#567](https://github.com/dfinity/ic/pull/567)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`21c75cb41`](https://github.com/dfinity/ic/commit/21c75cb41) Interface,Node: introduce release-pkg and ic-os-pkg package groups ([#553](https://github.com/dfinity/ic/pull/553)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Andr Popo | [`ac0bdf46b`](https://github.com/dfinity/ic/commit/ac0bdf46b) Message Routing(idx): Restore buf.yaml ([#450](https://github.com/dfinity/ic/pull/450)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Andr Popo | [`9724376a0`](https://github.com/dfinity/ic/commit/9724376a0) Message Routing(idx): Restore buf.yaml ([#376](https://github.com/dfinity/ic/pull/376)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Leo  Eich | [`ef680f5f2`](https://github.com/dfinity/ic/commit/ef680f5f2) Message Routing: Stop ignoring `consensus.proto` in `buf.yaml` [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`2c0b76cfc`](https://github.com/dfinity/ic/commit/2c0b76cfc) Owners(IDX): updating container autobuild ([#390](https://github.com/dfinity/ic/pull/390)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Niko Milo | [`864666555`](https://github.com/dfinity/ic/commit/864666555) Owners: adding back xnet test canister ([#624](https://github.com/dfinity/ic/pull/624)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`8c3f19ca5`](https://github.com/dfinity/ic/commit/8c3f19ca5) Owners(IDX): remove gitlab tests completely ([#519](https://github.com/dfinity/ic/pull/519)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Luka Skug | [`7c5e06583`](https://github.com/dfinity/ic/commit/7c5e06583) Owners: revert "remove binaries which don't need to be released (e.g. stripped) and don't need to to uploaded to the CDN" ([#616](https://github.com/dfinity/ic/pull/616)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`a98f5949b`](https://github.com/dfinity/ic/commit/a98f5949b) Owners(IDX): k8s system tests schedule ([#599](https://github.com/dfinity/ic/pull/599)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`9c36d497b`](https://github.com/dfinity/ic/commit/9c36d497b) Owners: remove binaries which don't need to be released (e.g. stripped) and don't need to to uploaded to the CDN ([#563](https://github.com/dfinity/ic/pull/563)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Fran Prei | [`e660c9b08`](https://github.com/dfinity/ic/commit/e660c9b08) Owners(crypto): adapt ic-signature-verification changelog ([#566](https://github.com/dfinity/ic/pull/566)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dimi Sarl | [`24c8384d0`](https://github.com/dfinity/ic/commit/24c8384d0) Owners: Move drun ownership to the languages team ([#565](https://github.com/dfinity/ic/pull/565)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`48cce3b3c`](https://github.com/dfinity/ic/commit/48cce3b3c) Owners(IDX): k8s system tests workflow ([#498](https://github.com/dfinity/ic/pull/498)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`a9ded4162`](https://github.com/dfinity/ic/commit/a9ded4162) Owners(IDX): update dependencies of //hs/spec_compliance:ic-ref-test ([#547](https://github.com/dfinity/ic/pull/547)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Stef Schn | [`540554986`](https://github.com/dfinity/ic/commit/540554986) Owners: Code owners for state manager config ([#522](https://github.com/dfinity/ic/pull/522)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`dcd24d6be`](https://github.com/dfinity/ic/commit/dcd24d6be) Owners: /rs/state_machine_tests/ owned by pocket-ic ([#521](https://github.com/dfinity/ic/pull/521)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`3ea305b03`](https://github.com/dfinity/ic/commit/3ea305b03) Owners(IDX): remove targets from rust_canister ([#440](https://github.com/dfinity/ic/pull/440)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`ad207fddb`](https://github.com/dfinity/ic/commit/ad207fddb) Owners(IDX): release testing update ([#464](https://github.com/dfinity/ic/pull/464)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`3a2aa6de8`](https://github.com/dfinity/ic/commit/3a2aa6de8) Owners(IDX): drop the --github suffix for rc branches ([#476](https://github.com/dfinity/ic/pull/476)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`36725229c`](https://github.com/dfinity/ic/commit/36725229c) Owners(IDX): update the node PR notification channel ([#474](https://github.com/dfinity/ic/pull/474)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Nico Matt | [`c5121e693`](https://github.com/dfinity/ic/commit/c5121e693) Owners(IDX): split .bazelrc ([#459](https://github.com/dfinity/ic/pull/459)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`bc3a6c321`](https://github.com/dfinity/ic/commit/bc3a6c321) Owners(IDX): changing schedule ([#456](https://github.com/dfinity/ic/pull/456)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`163b01f76`](https://github.com/dfinity/ic/commit/163b01f76) Owners(IDX): rm unneeded workflows ([#445](https://github.com/dfinity/ic/pull/445)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`d13d77482`](https://github.com/dfinity/ic/commit/d13d77482) Owners(IDX): update codeowners ([#443](https://github.com/dfinity/ic/pull/443)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: mras      | [`68aa7fc30`](https://github.com/dfinity/ic/commit/68aa7fc30) Owners: optimize starting PocketIC server in PocketIC library ([#439](https://github.com/dfinity/ic/pull/439)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`5c1ddd3bf`](https://github.com/dfinity/ic/commit/5c1ddd3bf) Owners(IDX): update release-alerts channel ([#434](https://github.com/dfinity/ic/pull/434)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`e38437aa3`](https://github.com/dfinity/ic/commit/e38437aa3) Owners(IDX): add sycing from public to private repo ([#429](https://github.com/dfinity/ic/pull/429)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`374c13181`](https://github.com/dfinity/ic/commit/374c13181) Owners(IDX): exclude workflows with long wait times ([#425](https://github.com/dfinity/ic/pull/425)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`c877909e6`](https://github.com/dfinity/ic/commit/c877909e6) Owners(IDX): updating base images workflow ([#366](https://github.com/dfinity/ic/pull/366)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`76ef61bc3`](https://github.com/dfinity/ic/commit/76ef61bc3) Owners(IDX): introduce the pocket-ic team and let it own the pocket-ic code ([#386](https://github.com/dfinity/ic/pull/386)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`becd51568`](https://github.com/dfinity/ic/commit/becd51568) Owners(IDX): add eng-cross-chain team ([#355](https://github.com/dfinity/ic/pull/355)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: venk      | [`72b90db5b`](https://github.com/dfinity/ic/commit/72b90db5b) Owners(dependency-mgmt): Migrate dependency management jobs to Github ([#332](https://github.com/dfinity/ic/pull/332)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`dd5c67525`](https://github.com/dfinity/ic/commit/dd5c67525) Owners(IDX): remove change base branch [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`66340d3f6`](https://github.com/dfinity/ic/commit/66340d3f6) Owners(IDX): dont run macos on merge queue [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Bas  van  | [`3b0cfd257`](https://github.com/dfinity/ic/commit/3b0cfd257) Owners(IDX): let infrasec co-own .github/workflows so they're notified about CI changes [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`775295f34`](https://github.com/dfinity/ic/commit/775295f34) Owners(IDX): disable schedule on CI Main [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`0e8cc5d47`](https://github.com/dfinity/ic/commit/0e8cc5d47) Owners(IDX): updating filtered tags [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mart Rasz | [`4d4fccc41`](https://github.com/dfinity/ic/commit/4d4fccc41) Owners: update serde_json in Cargo.toml according to bazel/external_crates.bzl [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`d2f891dd0`](https://github.com/dfinity/ic/commit/d2f891dd0) Owners(IDX): trigger on push to dev-gh-* [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`622fd758d`](https://github.com/dfinity/ic/commit/622fd758d) Owners(IDX): switch to ghcr.io [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Mark Kosm | [`8273f44aa`](https://github.com/dfinity/ic/commit/8273f44aa) Owners(IDX): container options [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: push      | [`7e2403507`](https://github.com/dfinity/ic/commit/7e2403507) Owners(github-sync): PR#327 / chore(IDX): only run certain python tests on security-hotfix [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`5428a9b29`](https://github.com/dfinity/ic/commit/5428a9b29) Owners(IDX): Update GitHub team [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Toma Hlav | [`03351c23f`](https://github.com/dfinity/ic/commit/03351c23f) Node: Align CH1 IPv6 prefix ([#489](https://github.com/dfinity/ic/pull/489)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Eero Kell | [`c69c755d5`](https://github.com/dfinity/ic/commit/c69c755d5) Node: Update ansible for upcoming lints [S3_UPLOAD] ([#422](https://github.com/dfinity/ic/pull/422)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Eero Kell | [`008a73566`](https://github.com/dfinity/ic/commit/008a73566) Node,Runtime: Miscellaneous updates in prep for upgrade ([#423](https://github.com/dfinity/ic/pull/423)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Maks Arut | [`76882bc40`](https://github.com/dfinity/ic/commit/76882bc40) Runtime: cleanup Bazel files ([#499](https://github.com/dfinity/ic/pull/499)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Math Björ | [`2e8fa1ad7`](https://github.com/dfinity/ic/commit/2e8fa1ad7) Interface(icp_ledger): Move test helper functions to test utils ([#462](https://github.com/dfinity/ic/pull/462)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: max-      | [`d04d4bbd5`](https://github.com/dfinity/ic/commit/d04d4bbd5) Interface(nns): no longer generate api types from internal protos ([#588](https://github.com/dfinity/ic/pull/588)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`5e34a0f8f`](https://github.com/dfinity/ic/commit/5e34a0f8f) Interface: introduce a system-tests-pkg group and add all system tests under that package group ([#528](https://github.com/dfinity/ic/pull/528)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`214998263`](https://github.com/dfinity/ic/commit/214998263) Interface: add testonly tag for some test libraries [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`fbffe8109`](https://github.com/dfinity/ic/commit/fbffe8109) Interface: remove unused rs/local-bin and update codeowners file [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Venk Seka | [`34ff2857a`](https://github.com/dfinity/ic/commit/34ff2857a) Interface,Runtime(fuzzing): create new test library `wasm_fuzzers` [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`417e0b5c1`](https://github.com/dfinity/ic/commit/417e0b5c1) Owners(IDX): remove gitlab reference from tests ([#520](https://github.com/dfinity/ic/pull/520)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`4c2c76525`](https://github.com/dfinity/ic/commit/4c2c76525) Owners(IDX): simplify hotfix branch matching logic ([#514](https://github.com/dfinity/ic/pull/514)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`77ee36864`](https://github.com/dfinity/ic/commit/77ee36864) Networking: update deny.toml [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Fran Prei | [`65f6d7dd0`](https://github.com/dfinity/ic/commit/65f6d7dd0) Consensus,Interface(orchestrator): increase subnet size for sr_app_large_with_tecdsa_test to 37 ([#586](https://github.com/dfinity/ic/pull/586)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Kami Popi | [`373c9f93f`](https://github.com/dfinity/ic/commit/373c9f93f) Consensus,Interface(consensus): Add artificial network restrictions to `consensus_performance_test` & print some throughput information at the end of the test [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Drag Djur | [`de3425fa6`](https://github.com/dfinity/ic/commit/de3425fa6) Execution,Interface,Runtime: make system api test to be state machine test ([#377](https://github.com/dfinity/ic/pull/377)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`3e4a107f6`](https://github.com/dfinity/ic/commit/3e4a107f6) Interface: stop uploading test canister artifacts  ([#533](https://github.com/dfinity/ic/pull/533)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Ulan Dege | [`bc8db7683`](https://github.com/dfinity/ic/commit/bc8db7683) Interface: Remove the scalability benchmarking suite ([#527](https://github.com/dfinity/ic/pull/527)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Math Björ | [`f2f408333`](https://github.com/dfinity/ic/commit/f2f408333) Interface(ICRC-Ledger): Add tests for upgrading ICRC ledger with WASMs with different token types ([#388](https://github.com/dfinity/ic/pull/388)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Math Björ | [`620613591`](https://github.com/dfinity/ic/commit/620613591) Interface(icrc_ledger): Upgrade test for ledgers using golden state ([#399](https://github.com/dfinity/ic/pull/399)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: dani      | [`2d2f3b550`](https://github.com/dfinity/ic/commit/2d2f3b550) Interface(sns): SNS upgrade-related tests were flaking out. ([#391](https://github.com/dfinity/ic/pull/391)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Leo  Eich | [`a67c3df73`](https://github.com/dfinity/ic/commit/a67c3df73) Interface: Generalize tECDSA performance test for tSchnorr and update dashboard [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Shar | [`530cc0b56`](https://github.com/dfinity/ic/commit/530cc0b56) Interface(execution-benchmark): Created a system test for benchmarking full execution rounds [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Ognj Mari | [`38c7a5098`](https://github.com/dfinity/ic/commit/38c7a5098) Interface,Message Routing: check canister queue upgrade/downgrade compatibility against published version [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`3d42215df`](https://github.com/dfinity/ic/commit/3d42215df) Owners: don't upload malicious_replica ([#538](https://github.com/dfinity/ic/pull/538)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`022619a77`](https://github.com/dfinity/ic/commit/022619a77) Owners: don't upload test canister artifacts ([#531](https://github.com/dfinity/ic/pull/531)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: dani      | [`d35c2fc45`](https://github.com/dfinity/ic/commit/d35c2fc45) Interface(nns): Adds a README to nns/governance. ([#325](https://github.com/dfinity/ic/pull/325)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Wong | [`7e1b19dba`](https://github.com/dfinity/ic/commit/7e1b19dba) Interface(nns): Changed help string of some neurons metrics to say that GTC is excluded. [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`6bb7d0143`](https://github.com/dfinity/ic/commit/6bb7d0143) Owners: rust external dep policy ([#358](https://github.com/dfinity/ic/pull/358)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`cd8f32f03`](https://github.com/dfinity/ic/commit/cd8f32f03) Networking: fix deny.toml [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Rost Rume | [`7c4a08fc2`](https://github.com/dfinity/ic/commit/7c4a08fc2) Node: why GuestOS deps are required ([#410](https://github.com/dfinity/ic/pull/410)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Math Björ | [`364fe4f38`](https://github.com/dfinity/ic/commit/364fe4f38) Interface: test(icp_ledger):, Get and query all blocks from ledger and archives and fix test_archive_indexing ([#398](https://github.com/dfinity/ic/pull/398)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Dani Wong | [`15beeb6a9`](https://github.com/dfinity/ic/commit/15beeb6a9) Interface(nns): Add and use workspace version of prometheus-parse. [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: Carl Gund | [`439491c53`](https://github.com/dfinity/ic/commit/439491c53) Owners: "fix(IDX): exclude certain tests from the merge train" ([#580](https://github.com/dfinity/ic/pull/580)) [AUTO-EXCLUDED:not a GuestOS change]~~
* ~~author: push      | [`10e396728`](https://github.com/dfinity/ic/commit/10e396728) Owners: chore(github-sync): PR#326 / chore(IDX): update repo reference in build-ic [AUTO-EXCLUDED:not a GuestOS change]~~
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
* author: Alex      | [`f0093242d`](https://github.com/dfinity/ic/commit/f0093242d) Execution,Interface: Enforce taking a canister snapshot only when canister is not empty ([#452](https://github.com/dfinity/ic/pull/452))
* author: Alex Uta  | [`736beea98`](https://github.com/dfinity/ic/commit/736beea98) Execution,Interface,Message Routing: Enable transparent huge pages for the page allocator ([#665](https://github.com/dfinity/ic/pull/665))
* author: Dimi Sarl | [`96035ca4c`](https://github.com/dfinity/ic/commit/96035ca4c) Execution,Interface,Networking: Reduce DTS slice limit for regular messages on system subnets ([#621](https://github.com/dfinity/ic/pull/621))
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
* author: Maci Kot  | [`f6a88d1a5`](https://github.com/dfinity/ic/commit/f6a88d1a5) Execution,Interface: Saturate function index in system api calls ([#641](https://github.com/dfinity/ic/pull/641))
* author: Drag Djur | [`4bebd6f6a`](https://github.com/dfinity/ic/commit/4bebd6f6a) Execution,Interface: Add Wasm memory threshold field to canister settings ([#475](https://github.com/dfinity/ic/pull/475))
* author: Ulan Dege | [`3e9785f87`](https://github.com/dfinity/ic/commit/3e9785f87) Execution,Interface: Rename fees_and_limits to icp_config ([#638](https://github.com/dfinity/ic/pull/638))
* ~~author: Andr Popo | [`9bc6e18ac`](https://github.com/dfinity/ic/commit/9bc6e18ac) Interface(neurons_fund): Populate hotkeys when necessary in the NNS Governance → Swap → SNS Governance dataflow ([#688](https://github.com/dfinity/ic/pull/688)) [AUTO-EXCLUDED]~~
* author: Dani Shar | [`b4be567dc`](https://github.com/dfinity/ic/commit/b4be567dc) Interface: Bump rust version to 1.80 ([#642](https://github.com/dfinity/ic/pull/642))
* author: mras      | [`dbfbeceea`](https://github.com/dfinity/ic/commit/dbfbeceea) Interface: bump jemallocator v0.3 to tikv-jemallocator v0.5 ([#654](https://github.com/dfinity/ic/pull/654))
* author: Leo  Eich | [`668fbe08f`](https://github.com/dfinity/ic/commit/668fbe08f) Interface: Rename ECDSA metrics ([#535](https://github.com/dfinity/ic/pull/535))
* ~~author: Dani Shar | [`219655bf7`](https://github.com/dfinity/ic/commit/219655bf7) Interface: Update `agent-rs` dependency version to 0.37.1 ([#560](https://github.com/dfinity/ic/pull/560)) [AUTO-EXCLUDED]~~
* author: Rost Rume | [`ec01b3735`](https://github.com/dfinity/ic/commit/ec01b3735) Interface: add tools-pkg ([#584](https://github.com/dfinity/ic/pull/584))
* author: Dimi Sarl | [`0527e6f50`](https://github.com/dfinity/ic/commit/0527e6f50) Interface,Message Routing: Use a single sentence for error messages in IngressInductionError ([#648](https://github.com/dfinity/ic/pull/648))
* author: Rost Rume | [`173d06185`](https://github.com/dfinity/ic/commit/173d06185) Interface,Node: build and strip IC-OS tools iff we build the VMs ([#609](https://github.com/dfinity/ic/pull/609))
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
