import typing

import commit_annotation
import const
import dryrun
import git_repo
import pydantic_yaml
import pytest_mock.plugin
from dre_cli import ElectionProposal
from public_dashboard import DashboardAPI
from reconciler import Reconciler
from reconciler_state import ReconcilerState
from release_index import Model as ReleaseIndexModel
from release_index_loader import StaticReleaseLoader
from tests.fixtures import ic_repo as ic_repo


class MockDashboard(DashboardAPI):
    """Reconciler state that uses static proposal data."""

    def __init__(self) -> None:
        super().__init__()

    def _fake_proposal(
        self,
        proposal_id: int,
        commit_id: str,
        short_commit_id: str,
        hash: str,
        os_type: const.OsKind,
    ) -> ElectionProposal:
        return {
            "id": proposal_id,
            "payload": {
                "hostos_version_to_elect": commit_id,
                "release_package_sha256_hex": hash,
            },
            "proposal_timestamp_seconds": 1743789296,
            "proposer": 61,
            "status": "EXECUTED",
            "summary": "...stubbed out...",
            "title": f"Elect new IC/{os_type} revision (commit {short_commit_id})",
        }

    def get_past_election_proposals(self) -> list[ElectionProposal]:
        return [
            self._fake_proposal(
                138817,
                "45657852c1eca6728ff313808db29b47c862ad13",
                "4565785",
                "f7ee7bf6218fbc9938175c79e7bb7b3183215d4faedea23b628db8d247a3ef60",
                const.HOSTOS,
            ),
            self._fake_proposal(
                138814,
                "45657852c1eca6728ff313808db29b47c862ad13",
                "4565785",
                "84a17802d839e057727ff09e34f2cba47c129e7ca18f33ed38dbf99740809808",
                const.GUESTOS,
            ),
            self._fake_proposal(
                138817,
                "206b61a8616bc93d36d6a014e5cc8edf1ba256ae",
                "206b61a",
                "71c39a3943a4c5e19884463766af9f29528c9ea7aa7363fa452599d7b95d4e76",
                const.HOSTOS,
            ),
            self._fake_proposal(
                138814,
                "206b61a8616bc93d36d6a014e5cc8edf1ba256ae",
                "206b61a",
                "6b4965857e181ce9508f879cba56a6425e0e441ad1bb73f56b11dcaf247bd4eb",
                const.GUESTOS,
            ),
        ]


class MockActiveVersionProvider(object):
    def __init__(self, active_versions: list[str] | None = None):
        self.vers = active_versions if active_versions else []

    def active_guestos_versions(self) -> list[str]:
        return self.vers

    def active_hostos_versions(self) -> list[str]:
        return self.vers


def _release(rc_name: str, versions: dict[str, str]) -> dict[str, typing.Any]:
    return {
        "rc_name": rc_name,
        "versions": [{"name": k, "version": v} for k, v in versions.items()],
    }


def _defaults_for_test(
    releases: list[dict[str, typing.Any]] = [
        _release(
            "rc--2025-10-02_03-13", {"base": "45657852c1eca6728ff313808db29b47c862ad13"}
        ),
        _release(
            "rc--2025-09-25_09-52", {"base": "206b61a8616bc93d36d6a014e5cc8edf1ba256ae"}
        ),
        _release(
            "rc--2025-09-19_10-17", {"base": "bf0d4d1b8cb6c0c19a5afa1454ada014847aa5c6"}
        ),
    ],
) -> tuple[
    dryrun.StubDiscourseClient,
    dryrun.ForumClient,
    dryrun.ReleaseNotesClient,
    ReconcilerState,
    MockActiveVersionProvider,
    dryrun.DRECli,
    dryrun.MockSlackAnnouncer,
    StaticReleaseLoader,
    dryrun.PublishNotesClient,
    MockDashboard,
]:
    discourse_client = dryrun.StubDiscourseClient()
    return (
        discourse_client,
        dryrun.ForumClient(discourse_client),
        dryrun.ReleaseNotesClient(),
        ReconcilerState(),
        MockActiveVersionProvider(),
        dryrun.DRECli(),
        dryrun.MockSlackAnnouncer(),
        StaticReleaseLoader(
            pydantic_yaml.to_yaml_str(
                ReleaseIndexModel.model_validate({"releases": releases})
            )
        ),
        dryrun.PublishNotesClient(),
        MockDashboard(),
    )


def _cdf(r: git_repo.GitRepo) -> commit_annotation.ChangeDeterminatorProtocol:
    return commit_annotation.LocalCommitChangeDeterminator(r)


def test_reconciler_reconciles_without_error_already_submitted_proposals(
    ic_repo: git_repo.GitRepo,
    mocker: pytest_mock.plugin.MockerFixture,
) -> None:
    """
    Exercise the reconciler and ensure it works end to end without failure
    when the index contains two already-published releases.
    """
    with mocker.patch.object(ic_repo, "push_release_tags"):
        d, f, n, rs, a, dre, s, rl, p, db = _defaults_for_test()
        reconciler = Reconciler(
            f, rl, n, p, "", rs, ic_repo, lambda: _cdf(ic_repo), a, dre, db, s
        )

        def fake_approved_release_notes(*args):  # type: ignore
            return f"Fake changelog for {args}"

        rl.proposal_summary = fake_approved_release_notes  # type: ignore
        reconciler.reconcile()
        guestos_post = d.topics[0]["post_stream"]["posts"][0]
        hostos_post = d.topics[0]["post_stream"]["posts"][1]
        expected_guestos_post = """Hello there!

We are happy to announce that voting is now open for [a new GuestOS release](https://github.com/dfinity/ic/tree/release-2025-09-25_09-52-base).
The NNS proposal is here: [IC NNS Proposal 138708](https://dashboard.internetcomputer.org/proposal/138708).

Here is a summary of the changes since the last GuestOS release:

Fake changelog for ('206b61a8616bc93d36d6a014e5cc8edf1ba256ae', 'GuestOS', False)"""
        expected_hostos_post = """Hello there!

We are happy to announce that voting is now open for [a new HostOS release](https://github.com/dfinity/ic/tree/release-2025-09-25_09-52-base).
The NNS proposal is here: [IC NNS Proposal 138814](https://dashboard.internetcomputer.org/proposal/138814).

Here is a summary of the changes since the last HostOS release:

Fake changelog for ('206b61a8616bc93d36d6a014e5cc8edf1ba256ae', 'HostOS', False)"""

        assert guestos_post["raw"] == expected_guestos_post
        assert hostos_post["raw"] == expected_hostos_post


def test_reconciler_publishes_tentative_changelog_when_changelog_not_yet_approved(
    ic_repo: git_repo.GitRepo,
    mocker: pytest_mock.plugin.MockerFixture,
) -> None:
    """
    A new release added to the index should cause release notes generation
    and posting prior to the release notes PRs being generated.
    """
    with mocker.patch.object(ic_repo, "push_release_tags"):
        releases = [
            # The following is a fake release that does not exist yet.
            # We want to test that the reconciler produces and publishes
            # draft release notes for both HostOS and GuestOS.
            _release(
                "rc--2025-10-12_01-01",
                {"base": "891c0d9d63b158792f68999a69ad597e6c9130ff"},
            ),
            _release(
                "rc--2025-10-02_03-13",
                {"base": "45657852c1eca6728ff313808db29b47c862ad13"},
            ),
            _release(
                "rc--2025-09-25_09-52",
                {"base": "206b61a8616bc93d36d6a014e5cc8edf1ba256ae"},
            ),
        ]
        d, f, n, rs, a, dre, s, rl, p, db = _defaults_for_test(releases=releases)
        reconciler = Reconciler(
            f, rl, n, p, "", rs, ic_repo, lambda: _cdf(ic_repo), a, dre, db, s
        )
        reconciler.reconcile()

        guestos_post = d.topics[1]["post_stream"]["posts"][0]
        hostos_post = d.topics[1]["post_stream"]["posts"][1]
        # First published draft is for GuestOS.
        assert guestos_post["cooked"].startswith(
            "We're preparing [a new IC "
            "release](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base).\n"
            "\n"
            "The following is a **draft** of the list of changes since the last GuestOS "
            "release:\n"
            "\n"
        ), guestos_post["cooked"]
        # Second published draft is for HostOS.
        assert hostos_post["cooked"].startswith(
            "We're preparing [a new IC "
            "release](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base).\n"
            "\n"
            "The following is a **draft** of the list of changes since the last HostOS "
            "release:\n"
            "\n"
        ), hostos_post["cooked"]

        expected_guestos_release_notes = """We're preparing [a new IC release](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base).

The following is a **draft** of the list of changes since the last GuestOS release:

# Release Notes for [release-2025-10-12_01-01-base](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base) (`891c0d9d63b158792f68999a69ad597e6c9130ff`)
This release is based on changes since [release-2025-10-02_03-13-base](https://dashboard.internetcomputer.org/release/45657852c1eca6728ff313808db29b47c862ad13) (`45657852c1eca6728ff313808db29b47c862ad13`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2025-10-02_03-13-base...release-2025-10-12_01-01-base).
## Features:
## Bugfixes:
* [`f7d0a8f47`](https://github.com/dfinity/ic/commit/f7d0a8f47) Consensus(orchestrator): abort initial sleep in hostOS upgrade checks if requested ([#7046](https://github.com/dfinity/ic/pull/7046))
* [`04444c85b`](https://github.com/dfinity/ic/commit/04444c85b) Execution: subnet memory taken for canisters with memory allocation ([#7028](https://github.com/dfinity/ic/pull/7028))
* [`a54a2fd77`](https://github.com/dfinity/ic/commit/a54a2fd77) Node: revert "chore: Resize backup LV to 500GB ([#6926](https://github.com/dfinity/ic/pull/6926))" ([#7059](https://github.com/dfinity/ic/pull/7059))
## Performance improvements:
* [`a9ea00d62`](https://github.com/dfinity/ic/commit/a9ea00d62) Consensus(idkg): Use multi-threading in `send_dealing_support` ([#6968](https://github.com/dfinity/ic/pull/6968))
* [`5fdc757d6`](https://github.com/dfinity/ic/commit/5fdc757d6) Consensus(idkg): Use multi-threading in `validate_dealings` ([#6962](https://github.com/dfinity/ic/pull/6962))
## Chores:
* [`67cc97592`](https://github.com/dfinity/ic/commit/67cc97592) Consensus: Shorten NNS delegation refresh interval ([#7068](https://github.com/dfinity/ic/pull/7068))
* [`17f72f06c`](https://github.com/dfinity/ic/commit/17f72f06c) Execution: consider memory allocation in scheduler invariants ([#7042](https://github.com/dfinity/ic/pull/7042))
* [`0945e0f03`](https://github.com/dfinity/ic/commit/0945e0f03) Execution: Improve error suggestion for CanisterMetadataSectionNotFound ([#7036](https://github.com/dfinity/ic/pull/7036))
* [`f9818d864`](https://github.com/dfinity/ic/commit/f9818d864) Execution: remove obsolete canister_log_memory_usage v1 metric ([#7011](https://github.com/dfinity/ic/pull/7011))
* [`ffd98c33d`](https://github.com/dfinity/ic/commit/ffd98c33d) Message Routing: rename ManifestDelta ([#7070](https://github.com/dfinity/ic/pull/7070))
* [`fa37988f9`](https://github.com/dfinity/ic/commit/fa37988f9) Message Routing: Remove old BitVec logic from incremental manifest computation ([#7052](https://github.com/dfinity/ic/pull/7052))
* [`741426fe1`](https://github.com/dfinity/ic/commit/741426fe1) Node: Drop ext4 support from monitor-expand-shared-data ([#7055](https://github.com/dfinity/ic/pull/7055))
* [`3d50e3e43`](https://github.com/dfinity/ic/commit/3d50e3e43) Node: Resize backup LV to 500GB ([#6926](https://github.com/dfinity/ic/pull/6926))
## Refactoring:
## Tests:

## Excluded Changes

### Changed files are excluded by file path filter
* [`1e4faccbc`](https://github.com/dfinity/ic/commit/1e4faccbc) Execution: unify management canister doc comments for Rust types ([#7062](https://github.com/dfinity/ic/pull/7062))

### Not modifying GuestOS
* [`b0059ae3f`](https://github.com/dfinity/ic/commit/b0059ae3f) Execution(sns-wasm): Add an option to skip updating latest version in SnsWasm::add_wasm ([#7058](https://github.com/dfinity/ic/pull/7058))
* [`ccacbf11c`](https://github.com/dfinity/ic/commit/ccacbf11c) Governance(nns): Archive topics of garbage collected proposals ([#7020](https://github.com/dfinity/ic/pull/7020))
* [`ff761f361`](https://github.com/dfinity/ic/commit/ff761f361) Governance(nns): Stop exposing KnownNeuronData in list_neurons ([#6953](https://github.com/dfinity/ic/pull/6953))
* [`5dcdf2ef8`](https://github.com/dfinity/ic/commit/5dcdf2ef8) Owners(dogecoin): facade for ckdoge minter canister ([#6814](https://github.com/dfinity/ic/pull/6814))
* [`8eba66ec7`](https://github.com/dfinity/ic/commit/8eba66ec7) Node: Track mainnet measurements in repo (again) ([#7022](https://github.com/dfinity/ic/pull/7022))
* [`79668f2e5`](https://github.com/dfinity/ic/commit/79668f2e5) Governance: add empty governance_test.did files to make cargo clippy succeed ([#7079](https://github.com/dfinity/ic/pull/7079))
* [`d7516b0b0`](https://github.com/dfinity/ic/commit/d7516b0b0) Governance(nervous-system-tools): Let proposal generation script use the right commit for reading changelogs ([#7076](https://github.com/dfinity/ic/pull/7076))
* [`0e08d8b07`](https://github.com/dfinity/ic/commit/0e08d8b07) Governance: recertify registry after canister migration ([#7040](https://github.com/dfinity/ic/pull/7040))
* [`ef525f001`](https://github.com/dfinity/ic/commit/ef525f001) IDX: temporarily adding repro-check back to tools ([#7067](https://github.com/dfinity/ic/pull/7067))
* [`d63c89bcb`](https://github.com/dfinity/ic/commit/d63c89bcb) Node: documentation file paths ([#7044](https://github.com/dfinity/ic/pull/7044))
* [`db66ec472`](https://github.com/dfinity/ic/commit/db66ec472) Consensus,Node(nns-recovery): reduce resource usage of NNS recovery system tests ([#7018](https://github.com/dfinity/ic/pull/7018))
* [`71237836a`](https://github.com/dfinity/ic/commit/71237836a) Execution: Remove dfn_macro ([#6922](https://github.com/dfinity/ic/pull/6922))
* [`891c0d9d6`](https://github.com/dfinity/ic/commit/891c0d9d6) Owners: Update Mainnet ICOS revisions file ([#7085](https://github.com/dfinity/ic/pull/7085))
* [`a44bcc6d2`](https://github.com/dfinity/ic/commit/a44bcc6d2) Owners: Update Mainnet ICOS revisions file ([#7083](https://github.com/dfinity/ic/pull/7083))
* [`0756b99d2`](https://github.com/dfinity/ic/commit/0756b99d2) IDX: bump oisy npm to 22.12 ([#7081](https://github.com/dfinity/ic/pull/7081))
* [`132f6ee3b`](https://github.com/dfinity/ic/commit/132f6ee3b) IDX: fix cargo build logic ([#7048](https://github.com/dfinity/ic/pull/7048))
* [`755aed257`](https://github.com/dfinity/ic/commit/755aed257) Node: use node reward type to determine node generation ([#6961](https://github.com/dfinity/ic/pull/6961))
* [`b0dc45feb`](https://github.com/dfinity/ic/commit/b0dc45feb) Node: Move tools onto config types ([#7019](https://github.com/dfinity/ic/pull/7019))
* [`cf07c0912`](https://github.com/dfinity/ic/commit/cf07c0912) Node: clean up nested test and improve code reuse ([#7017](https://github.com/dfinity/ic/pull/7017))
* [`fb59d8233`](https://github.com/dfinity/ic/commit/fb59d8233) Node: in the kill_start_test run the kill-start iteration 5 times ([#7050](https://github.com/dfinity/ic/pull/7050))
* [`c78222177`](https://github.com/dfinity/ic/commit/c78222177) Governance(nns/sns): Use patching for test canister candid files ([#6947](https://github.com/dfinity/ic/pull/6947))
* [`7c84f99be`](https://github.com/dfinity/ic/commit/7c84f99be) Execution: Systest for migration canister ([#7004](https://github.com/dfinity/ic/pull/7004))
* [`fd628eccb`](https://github.com/dfinity/ic/commit/fd628eccb) Financial Integrations(ICRC-Ledger): endpoint that disables icrc3 in the test ledger ([#7041](https://github.com/dfinity/ic/pull/7041))
* [`c027ae49c`](https://github.com/dfinity/ic/commit/c027ae49c) Node: duplicate kill_start_test into a long and short version ([#7060](https://github.com/dfinity/ic/pull/7060))

### The change is not owned by any replica or HostOS team
* [`a21d0e6a3`](https://github.com/dfinity/ic/commit/a21d0e6a3) Boundary Nodes: add aliases for request type variants for backwards compatibility ([#7032](https://github.com/dfinity/ic/pull/7032))
* [`fb4dff62d`](https://github.com/dfinity/ic/commit/fb4dff62d) Financial Integrations(icrc-ledger-types): Add try_from_subaccount_to_principal ([#6911](https://github.com/dfinity/ic/pull/6911))
""".rstrip()
        expected_hostos_release_notes = """We're preparing [a new IC release](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base).

The following is a **draft** of the list of changes since the last HostOS release:

# Release Notes for [release-2025-10-12_01-01-base](https://github.com/dfinity/ic/tree/release-2025-10-12_01-01-base) (`891c0d9d63b158792f68999a69ad597e6c9130ff`)
This release is based on changes since [release-2025-10-02_03-13-base](https://dashboard.internetcomputer.org/release/45657852c1eca6728ff313808db29b47c862ad13) (`45657852c1eca6728ff313808db29b47c862ad13`).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the HostOS image.
Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2025-10-02_03-13-base...release-2025-10-12_01-01-base).
## Features:
## Bugfixes:
## Performance improvements:
## Chores:
* [`755aed257`](https://github.com/dfinity/ic/commit/755aed257) Node: use node reward type to determine node generation ([#6961](https://github.com/dfinity/ic/pull/6961))
## Refactoring:
## Tests:

## Excluded Changes

### Changed files are excluded by file path filter
* [`1e4faccbc`](https://github.com/dfinity/ic/commit/1e4faccbc) Execution: unify management canister doc comments for Rust types ([#7062](https://github.com/dfinity/ic/pull/7062))

### Not modifying HostOS
* [`b0059ae3f`](https://github.com/dfinity/ic/commit/b0059ae3f) Execution(sns-wasm): Add an option to skip updating latest version in SnsWasm::add_wasm ([#7058](https://github.com/dfinity/ic/pull/7058))
* [`ccacbf11c`](https://github.com/dfinity/ic/commit/ccacbf11c) Governance(nns): Archive topics of garbage collected proposals ([#7020](https://github.com/dfinity/ic/pull/7020))
* [`ff761f361`](https://github.com/dfinity/ic/commit/ff761f361) Governance(nns): Stop exposing KnownNeuronData in list_neurons ([#6953](https://github.com/dfinity/ic/pull/6953))
* [`5dcdf2ef8`](https://github.com/dfinity/ic/commit/5dcdf2ef8) Owners(dogecoin): facade for ckdoge minter canister ([#6814](https://github.com/dfinity/ic/pull/6814))
* [`8eba66ec7`](https://github.com/dfinity/ic/commit/8eba66ec7) Node: Track mainnet measurements in repo (again) ([#7022](https://github.com/dfinity/ic/pull/7022))
* [`a21d0e6a3`](https://github.com/dfinity/ic/commit/a21d0e6a3) Boundary Nodes: add aliases for request type variants for backwards compatibility ([#7032](https://github.com/dfinity/ic/pull/7032))
* [`f7d0a8f47`](https://github.com/dfinity/ic/commit/f7d0a8f47) Consensus(orchestrator): abort initial sleep in hostOS upgrade checks if requested ([#7046](https://github.com/dfinity/ic/pull/7046))
* [`04444c85b`](https://github.com/dfinity/ic/commit/04444c85b) Execution: subnet memory taken for canisters with memory allocation ([#7028](https://github.com/dfinity/ic/pull/7028))
* [`fb4dff62d`](https://github.com/dfinity/ic/commit/fb4dff62d) Financial Integrations(icrc-ledger-types): Add try_from_subaccount_to_principal ([#6911](https://github.com/dfinity/ic/pull/6911))
* [`79668f2e5`](https://github.com/dfinity/ic/commit/79668f2e5) Governance: add empty governance_test.did files to make cargo clippy succeed ([#7079](https://github.com/dfinity/ic/pull/7079))
* [`d7516b0b0`](https://github.com/dfinity/ic/commit/d7516b0b0) Governance(nervous-system-tools): Let proposal generation script use the right commit for reading changelogs ([#7076](https://github.com/dfinity/ic/pull/7076))
* [`0e08d8b07`](https://github.com/dfinity/ic/commit/0e08d8b07) Governance: recertify registry after canister migration ([#7040](https://github.com/dfinity/ic/pull/7040))
* [`ef525f001`](https://github.com/dfinity/ic/commit/ef525f001) IDX: temporarily adding repro-check back to tools ([#7067](https://github.com/dfinity/ic/pull/7067))
* [`a54a2fd77`](https://github.com/dfinity/ic/commit/a54a2fd77) Node: revert "chore: Resize backup LV to 500GB ([#6926](https://github.com/dfinity/ic/pull/6926))" ([#7059](https://github.com/dfinity/ic/pull/7059))
* [`d63c89bcb`](https://github.com/dfinity/ic/commit/d63c89bcb) Node: documentation file paths ([#7044](https://github.com/dfinity/ic/pull/7044))
* [`a9ea00d62`](https://github.com/dfinity/ic/commit/a9ea00d62) Consensus(idkg): Use multi-threading in `send_dealing_support` ([#6968](https://github.com/dfinity/ic/pull/6968))
* [`5fdc757d6`](https://github.com/dfinity/ic/commit/5fdc757d6) Consensus(idkg): Use multi-threading in `validate_dealings` ([#6962](https://github.com/dfinity/ic/pull/6962))
* [`db66ec472`](https://github.com/dfinity/ic/commit/db66ec472) Consensus,Node(nns-recovery): reduce resource usage of NNS recovery system tests ([#7018](https://github.com/dfinity/ic/pull/7018))
* [`67cc97592`](https://github.com/dfinity/ic/commit/67cc97592) Consensus: Shorten NNS delegation refresh interval ([#7068](https://github.com/dfinity/ic/pull/7068))
* [`71237836a`](https://github.com/dfinity/ic/commit/71237836a) Execution: Remove dfn_macro ([#6922](https://github.com/dfinity/ic/pull/6922))
* [`17f72f06c`](https://github.com/dfinity/ic/commit/17f72f06c) Execution: consider memory allocation in scheduler invariants ([#7042](https://github.com/dfinity/ic/pull/7042))
* [`0945e0f03`](https://github.com/dfinity/ic/commit/0945e0f03) Execution: Improve error suggestion for CanisterMetadataSectionNotFound ([#7036](https://github.com/dfinity/ic/pull/7036))
* [`f9818d864`](https://github.com/dfinity/ic/commit/f9818d864) Execution: remove obsolete canister_log_memory_usage v1 metric ([#7011](https://github.com/dfinity/ic/pull/7011))
* [`ffd98c33d`](https://github.com/dfinity/ic/commit/ffd98c33d) Message Routing: rename ManifestDelta ([#7070](https://github.com/dfinity/ic/pull/7070))
* [`fa37988f9`](https://github.com/dfinity/ic/commit/fa37988f9) Message Routing: Remove old BitVec logic from incremental manifest computation ([#7052](https://github.com/dfinity/ic/pull/7052))
* [`891c0d9d6`](https://github.com/dfinity/ic/commit/891c0d9d6) Owners: Update Mainnet ICOS revisions file ([#7085](https://github.com/dfinity/ic/pull/7085))
* [`a44bcc6d2`](https://github.com/dfinity/ic/commit/a44bcc6d2) Owners: Update Mainnet ICOS revisions file ([#7083](https://github.com/dfinity/ic/pull/7083))
* [`0756b99d2`](https://github.com/dfinity/ic/commit/0756b99d2) IDX: bump oisy npm to 22.12 ([#7081](https://github.com/dfinity/ic/pull/7081))
* [`132f6ee3b`](https://github.com/dfinity/ic/commit/132f6ee3b) IDX: fix cargo build logic ([#7048](https://github.com/dfinity/ic/pull/7048))
* [`741426fe1`](https://github.com/dfinity/ic/commit/741426fe1) Node: Drop ext4 support from monitor-expand-shared-data ([#7055](https://github.com/dfinity/ic/pull/7055))
* [`3d50e3e43`](https://github.com/dfinity/ic/commit/3d50e3e43) Node: Resize backup LV to 500GB ([#6926](https://github.com/dfinity/ic/pull/6926))
* [`b0dc45feb`](https://github.com/dfinity/ic/commit/b0dc45feb) Node: Move tools onto config types ([#7019](https://github.com/dfinity/ic/pull/7019))
* [`cf07c0912`](https://github.com/dfinity/ic/commit/cf07c0912) Node: clean up nested test and improve code reuse ([#7017](https://github.com/dfinity/ic/pull/7017))
* [`fb59d8233`](https://github.com/dfinity/ic/commit/fb59d8233) Node: in the kill_start_test run the kill-start iteration 5 times ([#7050](https://github.com/dfinity/ic/pull/7050))
* [`c78222177`](https://github.com/dfinity/ic/commit/c78222177) Governance(nns/sns): Use patching for test canister candid files ([#6947](https://github.com/dfinity/ic/pull/6947))
* [`7c84f99be`](https://github.com/dfinity/ic/commit/7c84f99be) Execution: Systest for migration canister ([#7004](https://github.com/dfinity/ic/pull/7004))
* [`fd628eccb`](https://github.com/dfinity/ic/commit/fd628eccb) Financial Integrations(ICRC-Ledger): endpoint that disables icrc3 in the test ledger ([#7041](https://github.com/dfinity/ic/pull/7041))
* [`c027ae49c`](https://github.com/dfinity/ic/commit/c027ae49c) Node: duplicate kill_start_test into a long and short version ([#7060](https://github.com/dfinity/ic/pull/7060))
""".rstrip()
        assert guestos_post["cooked"] == expected_guestos_release_notes
        assert hostos_post["cooked"] == expected_hostos_release_notes
