# type: ignore

import pytest
import os
from github import Github
from publish_notes import PublishNotesClient
import pathlib
from google_docs import google_doc_to_markdown


# @pytest.mark.skip(
#    "Not functioning properly -- the checklist is devoid of team names, I am not sure what should happen in this case"
# )
def test_publish_if_ready__ready(mocker) -> None:
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    publish_client = PublishNotesClient(repo)
    mocker.patch.object(publish_client, "ensure_published")

    md = google_doc_to_markdown(
        pathlib.Path(os.path.dirname(__file__))
        / "test_data"
        / "b0ade55f7e8999e2842fe3f49df163ba224b71a2.docx"
    )
    publish_client.publish_if_ready(
        md,
        "b0ade55f7e8999e2842fe3f49df163ba224b71a2",
    )

    publish_client.ensure_published.assert_called_once_with(  # pylint: disable=no-member
        version="b0ade55f7e8999e2842fe3f49df163ba224b71a2",
        changelog="""\
Release Notes for [**release-2024-08-21\\_15-36-base**](https://github.com/dfinity/ic/tree/release-2024-08-21_15-36-base) (b0ade55f7e8999e2842fe3f49df163ba224b71a2)
===================================================================================================================================================================

This release is based on changes since [release-2024-08-15\\_01-30-base](https://dashboard.internetcomputer.org/release/6968299131311c836917f0d16d0b1b963526c9b1) (6968299131311c836917f0d16d0b1b963526c9b1).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-08-15_01-30-base...release-2024-08-21_15-36-base).

Features:
---------

* [`0d99d0d81`](https://github.com/dfinity/ic/commit/0d99d0d81) Consensus,Interface(consensus): Implement artifact pool bounds for equivocation proofs ([#974](https://github.com/dfinity/ic/pull/974))
* [`faacac310`](https://github.com/dfinity/ic/commit/faacac310) Consensus,Interface(consensus): Purge equivocation proofs below and at finalized height ([#927](https://github.com/dfinity/ic/pull/927))
* [`20e19f491`](https://github.com/dfinity/ic/commit/20e19f491) Crypto: remove ic-signature-verification's dependency on rand ([#994](https://github.com/dfinity/ic/pull/994))
* [`4897fd255`](https://github.com/dfinity/ic/commit/4897fd255) Interface(PocketIC): subnet read state requests ([#968](https://github.com/dfinity/ic/pull/968))
* [`2d220277b`](https://github.com/dfinity/ic/commit/2d220277b) Interface: ValidateEq derivation and annotation ([#808](https://github.com/dfinity/ic/pull/808))

Bugfixes:
---------

* [`2998e00de`](https://github.com/dfinity/ic/commit/2998e00de) Consensus,Interface: remove the attribute concept ([#392](https://github.com/dfinity/ic/pull/392))
* [`dada69e8f`](https://github.com/dfinity/ic/commit/dada69e8f) Interface: method not supported error message ([#921](https://github.com/dfinity/ic/pull/921))

Performance improvements:
-------------------------

* [`6ec7a4977`](https://github.com/dfinity/ic/commit/6ec7a4977) Interface,Node: Optimize the inject\\_files tool ([#957](https://github.com/dfinity/ic/pull/957))
* [`8e149ef62`](https://github.com/dfinity/ic/commit/8e149ef62) Interface,Node: Optimize the implementation of dflate and add a test ([#954](https://github.com/dfinity/ic/pull/954))

Chores:
-------

* [`510fcac2f`](https://github.com/dfinity/ic/commit/510fcac2f) Consensus,Interface: Introduce the ArtifactMutation type ([#929](https://github.com/dfinity/ic/pull/929))
* [`87b72bec4`](https://github.com/dfinity/ic/commit/87b72bec4) Consensus,Interface(IDX): remove custom lmdb-rkv build ([#905](https://github.com/dfinity/ic/pull/905))
* [`435bdcc9f`](https://github.com/dfinity/ic/commit/435bdcc9f) Consensus,Interface,Networking: purge before shares creation ([#882](https://github.com/dfinity/ic/pull/882))
* [`68586671c`](https://github.com/dfinity/ic/commit/68586671c) Consensus,Interface,Networking(consensus): [Con-1229] stash more shares ([#902](https://github.com/dfinity/ic/pull/902))
* [`db118af8a`](https://github.com/dfinity/ic/commit/db118af8a) Execution,Interface(consensus): [Con-1228] bound http outcalls ([#859](https://github.com/dfinity/ic/pull/859))
* [`234ca3809`](https://github.com/dfinity/ic/commit/234ca3809) Interface(PocketIC): do not use no\\_op\\_logger and MetricsRegistry::default in PocketIC ([#965](https://github.com/dfinity/ic/pull/965))
* [`b0aef30f1`](https://github.com/dfinity/ic/commit/b0aef30f1) Interface: upgrade deps ([#907](https://github.com/dfinity/ic/pull/907))
* [`7e53880dd`](https://github.com/dfinity/ic/commit/7e53880dd) Interface,Message Routing(crypto): Extend state\\_machine\\_tests to support Schnorr signatures ([#912](https://github.com/dfinity/ic/pull/912))
* [`71b025f32`](https://github.com/dfinity/ic/commit/71b025f32) Interface,Networking: remove DummySocket from quic transport ([#973](https://github.com/dfinity/ic/pull/973))
* [`c9e692e3d`](https://github.com/dfinity/ic/commit/c9e692e3d) Owners(ic): Bump ic-cdk to v0.13.5 ([#998](https://github.com/dfinity/ic/pull/998))
* [`fe29bbcca`](https://github.com/dfinity/ic/commit/fe29bbcca) Node: Fix a typo in SetupOS ([#1016](https://github.com/dfinity/ic/pull/1016))
* [`11a4f14d8`](https://github.com/dfinity/ic/commit/11a4f14d8) Node: Update Base Image Refs [2024-08-15-0808] ([#948](https://github.com/dfinity/ic/pull/948))

Refactoring:
------------

* [`dcbfc2217`](https://github.com/dfinity/ic/commit/dcbfc2217) Interface: don't pull the registry canister as part of the GuestOS ([#494](https://github.com/dfinity/ic/pull/494))
* [`16b8ecb3f`](https://github.com/dfinity/ic/commit/16b8ecb3f) Interface,Message Routing: Remove PageMapType::get\\_mut ([#925](https://github.com/dfinity/ic/pull/925))

Tests:
------

* [`84d011ca5`](https://github.com/dfinity/ic/commit/84d011ca5) Execution,Interface(EXE): Add more tests for Wasm memory limit ([#995](https://github.com/dfinity/ic/pull/995))
* [`3fa04ed34`](https://github.com/dfinity/ic/commit/3fa04ed34) Execution,Interface,Message Routing: Clean up CanisterQueues proptests ([#969](https://github.com/dfinity/ic/pull/969))

-------------------------------------------

## Excluded Changes

### Excluded by authors
* [`f04c0ce20`](https://github.com/dfinity/ic/commit/f04c0ce20) Execution,Interface,Message Routing: Fix bug in StreamsTesting fixture ([#1014](https://github.com/dfinity/ic/pull/1014))
* [`43c59b2ff`](https://github.com/dfinity/ic/commit/43c59b2ff) Consensus,Interface: Make Cannot report master public key changed metric warning less noisy ([#986](https://github.com/dfinity/ic/pull/986))

### filtered out by package filters
* [`366404d06`](https://github.com/dfinity/ic/commit/366404d06) Interface(nns): Add date filtering to list\\_node\\_provider\\_rewards ([#979](https://github.com/dfinity/ic/pull/979))
* [`af6561dc3`](https://github.com/dfinity/ic/commit/af6561dc3) Interface(nns): Add endpoint to get historical node provider rewards ([#941](https://github.com/dfinity/ic/pull/941))
* [`b4ccc86f8`](https://github.com/dfinity/ic/commit/b4ccc86f8) Interface(nns): Change InstallCode proposal to always return wasm\\_module\\_hash and arg\\_hash ([#937](https://github.com/dfinity/ic/pull/937))
* [`4039ea27e`](https://github.com/dfinity/ic/commit/4039ea27e) Consensus,Interface,Node: add a per-boundary-node rate-limit of 1000 update calls per second ([#922](https://github.com/dfinity/ic/pull/922))
* [`528e08c1f`](https://github.com/dfinity/ic/commit/528e08c1f) Execution,Interface,Message Routing: Convert proptests to test strategy ([#978](https://github.com/dfinity/ic/pull/978))
* [`2251ac411`](https://github.com/dfinity/ic/commit/2251ac411) Interface(nns): Make the comments on the topics and proposals consistent with NNS Dapp and ICP Dashboard ([#1003](https://github.com/dfinity/ic/pull/1003))
* [`1fd18580d`](https://github.com/dfinity/ic/commit/1fd18580d) Interface(ICP-Ledger): remove maximum number of accounts ([#972](https://github.com/dfinity/ic/pull/972))
* [`8e4ffb731`](https://github.com/dfinity/ic/commit/8e4ffb731) Interface(nns): Cleanup NNS Governance API type definitions ([#961](https://github.com/dfinity/ic/pull/961))
* [`9edfbdc4b`](https://github.com/dfinity/ic/commit/9edfbdc4b) Interface,Message Routing: Add snapshots to subnet split manifest test ([#975](https://github.com/dfinity/ic/pull/975))

### not a GuestOS change
* [`56551ce78`](https://github.com/dfinity/ic/commit/56551ce78) Consensus,Interface(ic-backup): Purge snapshots from the hot storage more aggresively ([#1008](https://github.com/dfinity/ic/pull/1008))
* [`63345d6a4`](https://github.com/dfinity/ic/commit/63345d6a4) Interface(PocketIC): specify replica log level of PocketIC instances ([#971](https://github.com/dfinity/ic/pull/971))
* [`c16696f93`](https://github.com/dfinity/ic/commit/c16696f93) Interface(ckerc20): NNS proposal to add ckEURC ([#946](https://github.com/dfinity/ic/pull/946))
* [`268967ec9`](https://github.com/dfinity/ic/commit/268967ec9) Interface(PocketIC): VerifiedApplication subnets ([#963](https://github.com/dfinity/ic/pull/963))
* [`96cf599a6`](https://github.com/dfinity/ic/commit/96cf599a6) Interface(ICP-Rosetta): add symbol check ([#884](https://github.com/dfinity/ic/pull/884))
* [`6621525c0`](https://github.com/dfinity/ic/commit/6621525c0) Interface(nns): Flag for SetVisibility Proposals. ([#887](https://github.com/dfinity/ic/pull/887))
* [`52a3d3659`](https://github.com/dfinity/ic/commit/52a3d3659) Interface(PocketIC): artificial delay in auto progress mode of PocketIC ([#970](https://github.com/dfinity/ic/pull/970))
* [`b92f83285`](https://github.com/dfinity/ic/commit/b92f83285) Owners: slack failover data store ([#697](https://github.com/dfinity/ic/pull/697))
* [`1ad0ad696`](https://github.com/dfinity/ic/commit/1ad0ad696) Owners: add ic-gateway to dependency scanning ([#964](https://github.com/dfinity/ic/pull/964))
* [`449066c40`](https://github.com/dfinity/ic/commit/449066c40) Consensus,Interface(IDX): Fix nix MacOs build for rocksdb dependency ([#993](https://github.com/dfinity/ic/pull/993))
* [`74dae345f`](https://github.com/dfinity/ic/commit/74dae345f) Crypto,Interface: fix crypto cargo build ([#934](https://github.com/dfinity/ic/pull/934))
* [`c7b8d3d8b`](https://github.com/dfinity/ic/commit/c7b8d3d8b) Interface(PocketIC): HTTP gateway crash ([#1029](https://github.com/dfinity/ic/pull/1029))
* [`688137852`](https://github.com/dfinity/ic/commit/688137852) Interface(PocketIC): HTTP gateway can handle requests with IP address hosts ([#1025](https://github.com/dfinity/ic/pull/1025))
* [`d5f514da6`](https://github.com/dfinity/ic/commit/d5f514da6) Interface: adjust metric names in p2p dashboard ([#933](https://github.com/dfinity/ic/pull/933))
* [`12d1e6e9d`](https://github.com/dfinity/ic/commit/12d1e6e9d) Interface,Networking: simulated network didn't correctly apply all tc filters ([#928](https://github.com/dfinity/ic/pull/928))
* [`b0ade55f7`](https://github.com/dfinity/ic/commit/b0ade55f7) Owners(PSEC): check environment in periodic job before running
* [`f72e44ad0`](https://github.com/dfinity/ic/commit/f72e44ad0) Owners: check first block if text field doesn't contain prefix ([#1034](https://github.com/dfinity/ic/pull/1034))
* [`b0c612da4`](https://github.com/dfinity/ic/commit/b0c612da4) Owners(IDX): syntax error workflow daily ([#1018](https://github.com/dfinity/ic/pull/1018))
* [`dc960ac1b`](https://github.com/dfinity/ic/commit/dc960ac1b) Owners(IDX): update darwin trigger logic ([#1013](https://github.com/dfinity/ic/pull/1013))
* [`6392b8eae`](https://github.com/dfinity/ic/commit/6392b8eae) Owners(IDX): add cache permissions [RUN\\_ALL\\_BAZEL\\_TARGETS] ([#984](https://github.com/dfinity/ic/pull/984))
* [`9bd0a407b`](https://github.com/dfinity/ic/commit/9bd0a407b) Owners(ci): Use .zst instead of .gz disk images in more places ([#958](https://github.com/dfinity/ic/pull/958))
* [`975199acb`](https://github.com/dfinity/ic/commit/975199acb) Owners(IDX): remove darwin container check ([#950](https://github.com/dfinity/ic/pull/950))
* [`b3ee4e736`](https://github.com/dfinity/ic/commit/b3ee4e736) Node: Remove dead boundary-guestos files ([#962](https://github.com/dfinity/ic/pull/962))
* [`df4aca5dd`](https://github.com/dfinity/ic/commit/df4aca5dd) Consensus,Node: Update Mainnet IC revisions file ([#1010](https://github.com/dfinity/ic/pull/1010))
* [`3340b3656`](https://github.com/dfinity/ic/commit/3340b3656) Crypto: bump ic-signature-verification version to 0.2 ([#1006](https://github.com/dfinity/ic/pull/1006))
* [`dbf9b25d1`](https://github.com/dfinity/ic/commit/dbf9b25d1) Interface(PocketIC): block in instance deletion until PocketIC is dropped ([#1030](https://github.com/dfinity/ic/pull/1030))
* [`f19e510e5`](https://github.com/dfinity/ic/commit/f19e510e5) Interface(ICP-Rosetta): icp rosetta database table consolidation ([#872](https://github.com/dfinity/ic/pull/872))
* [`3c7b7f2ca`](https://github.com/dfinity/ic/commit/3c7b7f2ca) Interface: Remove obsolete and unused deployment in NNS canister\\_ids.json ([#931](https://github.com/dfinity/ic/pull/931))
* [`1e5a4012d`](https://github.com/dfinity/ic/commit/1e5a4012d) Interface: optimize NNS canister builds again ([#952](https://github.com/dfinity/ic/pull/952))
* [`7b3981ca0`](https://github.com/dfinity/ic/commit/7b3981ca0) Owners(IDX): remove channel alerts ([#1033](https://github.com/dfinity/ic/pull/1033))
* [`545a018dc`](https://github.com/dfinity/ic/commit/545a018dc) Owners: Bump governance-canister / governance-canister\\_test compressed WASM size limit from 1.3 to 1.4 MB ([#1012](https://github.com/dfinity/ic/pull/1012))
* [`830d1b9f3`](https://github.com/dfinity/ic/commit/830d1b9f3) Owners(ic): bump ic-cdk v0.12 & v0.14 ([#1009](https://github.com/dfinity/ic/pull/1009))
* [`baeef4d7b`](https://github.com/dfinity/ic/commit/baeef4d7b) Owners(IDX): update namespace jobs to trigger on pull\\_request ([#996](https://github.com/dfinity/ic/pull/996))
* [`6b6c8477c`](https://github.com/dfinity/ic/commit/6b6c8477c) Owners(IDX): bazel --profile=profile.json ([#901](https://github.com/dfinity/ic/pull/901))
* [`6f444bdf4`](https://github.com/dfinity/ic/commit/6f444bdf4) Owners(IDX): Add languages team channel ([#989](https://github.com/dfinity/ic/pull/989))
* [`bf0c93467`](https://github.com/dfinity/ic/commit/bf0c93467) Owners(dependency-mgmt): Check node version compatibility before performing the scan ([#793](https://github.com/dfinity/ic/pull/793))
* [`ecf68e296`](https://github.com/dfinity/ic/commit/ecf68e296) Owners: set networking team as codeowner for network simulation module ([#945](https://github.com/dfinity/ic/pull/945))
* [`24d732eb1`](https://github.com/dfinity/ic/commit/24d732eb1) Interface(ckerc20): Simplify return type of eth\\_rpc::call ([#853](https://github.com/dfinity/ic/pull/853))
* [`9a68c3bcf`](https://github.com/dfinity/ic/commit/9a68c3bcf) Interface,Message Routing: Use Testing Constants as Subnet IDs in Messaging Integration Tests ([#936](https://github.com/dfinity/ic/pull/936))
* [`676c5448f`](https://github.com/dfinity/ic/commit/676c5448f) Interface(ICRC\\_ledger): Add downgrade to mainnet version for SNS ledgers ([#967](https://github.com/dfinity/ic/pull/967))
* [`039322fe3`](https://github.com/dfinity/ic/commit/039322fe3) Interface(consensus): Use the synchronous call-v3 agent for consensus performance test ([#910](https://github.com/dfinity/ic/pull/910))
* [`b388425da`](https://github.com/dfinity/ic/commit/b388425da) Interface(icrc\\_ledger): Add ledger state verification for golden state upgrade test of SNS ledger ([#720](https://github.com/dfinity/ic/pull/720))
* [`a2f7d24f4`](https://github.com/dfinity/ic/commit/a2f7d24f4) Interface,Networking(network-simulation): Increase transmission control buffers ([#908](https://github.com/dfinity/ic/pull/908))
""",
    )


def test_publish_if_ready__remove_empty_sections(mocker):
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    publish_client = PublishNotesClient(repo)
    mocker.patch.object(publish_client, "ensure_published")

    publish_client.publish_if_ready(
        """\
Review checklist
================

Please cross\\-out your team once you finished the review

* ~~@team-consensus~~
* ~~@team-crypto~~
* ~~@team-messaging~~
* ~~@team-networking~~
* ~~@node-team~~
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on GitHub.

Bugfixes:
---------

Features:
---------

* ~~author: Igor Novg |~~ [5f9e639d1](https://github.com/dfinity/ic/commit/5f9e639d1) ~~Boundary Nodes: remove njs~~
* ~~author: Igor Novg |~~ [eb7f3dc5c](https://github.com/dfinity/ic/commit/eb7f3dc5c) ~~Boundary Nodes: improve nginx performance~~
* author: Kami Popi | [26f30f055](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height

Tests:
------

Chores:
-------

* author: ~~Leo Eich | [b4673936a](https://github.com/dfinity/ic/commit/b4673936a) Consensus(ecdsa):~~ Make key\\_unmasked\\_ref in PreSignatureQuadrupleRef required
* author: Leo Eich | [b733f7043](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* ~~author: Leo Eich | [6a4d8962c](https://github.com/dfinity/ic/commit/6a4d8962c) Consensus(ecdsa): Make masked kappa config optional~~
* author: Leo Eich | [e76c5a374](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares

Something:
----------
* author: Leo Eich | [2d63da24c](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): ~~Add optional kappa\\_unmasked config to QuadrupleInCreation~~

Other:
------

## Excluded Changes:
------
""",
        "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
    )

    # assert publish_client.ensure_published.call_count == 1
    publish_client.ensure_published.assert_called_once_with(  # pylint: disable=no-member
        version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        changelog="""\
Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on GitHub.

Features:
---------

* [`26f30f055`](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height

Chores:
-------

* [`b733f7043`](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* [`e76c5a374`](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares

## Excluded Changes

### Excluded by authors
* [`5f9e639d1`](https://github.com/dfinity/ic/commit/5f9e639d1) Boundary Nodes: remove njs
* [`eb7f3dc5c`](https://github.com/dfinity/ic/commit/eb7f3dc5c) Boundary Nodes: improve nginx performance
* [`b4673936a`](https://github.com/dfinity/ic/commit/b4673936a) Consensus(ecdsa): Make key\\_unmasked\\_ref in PreSignatureQuadrupleRef required
* [`6a4d8962c`](https://github.com/dfinity/ic/commit/6a4d8962c) Consensus(ecdsa): Make masked kappa config optional
* [`2d63da24c`](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation
""",
    )


def test_publish_if_ready__not_ready1(mocker):
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    publish_client = PublishNotesClient(repo)
    mocker.patch.object(publish_client, "ensure_published")

    publish_client.publish_if_ready(
        """\
Review checklist
================

Please cross\\-out your team once you finished the review

* ~~@team-consensus~~
* ~~@team-crypto~~
* @team-execution
* ~~@team-messaging~~
* ~~@team-networking~~
* ~~@node-team~~
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on GitHub.

Features:
---------

* ~~author: Igor Novg |~~ [5f9e639d1](https://github.com/dfinity/ic/commit/5f9e639d1) ~~Boundary Nodes: remove njs~~
* ~~author: Igor Novg |~~ [eb7f3dc5c](https://github.com/dfinity/ic/commit/eb7f3dc5c) ~~Boundary Nodes: improve nginx performance~~
* author: Kami Popi | [26f30f055](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height
* author: ~~Leo Eich | [b4673936a](https://github.com/dfinity/ic/commit/b4673936a) Consensus(ecdsa):~~ Make key\\_unmasked\\_ref in PreSignatureQuadrupleRef required
* author: Leo Eich | [b733f7043](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* ~~author: Leo Eich | [6a4d8962c](https://github.com/dfinity/ic/commit/6a4d8962c) Consensus(ecdsa): Make masked kappa config optional~~
* author: Leo Eich | [e76c5a374](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares
* author: Leo Eich | [2d63da24c](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation
""",
        "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
    )

    assert publish_client.ensure_published.call_count == 0  # pylint: disable=no-member


def test_publish_if_ready__not_ready2(mocker):
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    publish_client = PublishNotesClient(repo)
    mocker.patch.object(publish_client, "ensure_published")

    publish_client.publish_if_ready(
        """\
Review checklist
================

Please cross-out your team once you finished the review

* ~~@team-consensus~~
* ~~@team-crypto~~
* ~~@team-execution~~
* ~~@team-messaging~~
* ~~@team-networking~~
* @node-team
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on GitHub.

Features:
---------

* ~~author: Igor Novg |~~ [5f9e639d1](https://github.com/dfinity/ic/commit/5f9e639d1) ~~Boundary Nodes: remove njs~~
* ~~author: Igor Novg |~~ [eb7f3dc5c](https://github.com/dfinity/ic/commit/eb7f3dc5c) ~~Boundary Nodes: improve nginx performance~~
* author: Kami Popi | [26f30f055](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height
* author: ~~Leo Eich | [b4673936a](https://github.com/dfinity/ic/commit/b4673936a) Consensus(ecdsa):~~ Make key\\_unmasked\\_ref in PreSignatureQuadrupleRef required
* author: Leo Eich | [b733f7043](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* ~~author: Leo Eich | [6a4d8962c](https://github.com/dfinity/ic/commit/6a4d8962c) Consensus(ecdsa): Make masked kappa config optional~~
* author: Leo Eich | [e76c5a374](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares
* author: Leo Eich | [2d63da24c](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation

## Excluded Changes:
---------
""",
        "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
    )

    assert publish_client.ensure_published.call_count == 0  # pylint: disable=no-member


def test_publish_if_ready__ready_no_changes(mocker):
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    publish_client = PublishNotesClient(repo)
    mocker.patch.object(publish_client, "ensure_published")

    with pytest.raises(ValueError):
        publish_client.publish_if_ready(
            """\
Review checklist
================

Please cross-out your team once you finished the review

* ~~@team-consensus~~
* ~~@team-crypto~~
* ~~@team-execution~~
* ~~@team-messaging~~
* ~~@team-networking~~
* ~~@node-team~~
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on GitHub.

## Features:
---------

* ~~author: Igor Novg |~~ [5f9e639d1](https://github.com/dfinity/ic/commit/5f9e639d1) ~~Boundary Nodes: remove njs~~
* ~~author: Igor Novg |~~ [eb7f3dc5c](https://github.com/dfinity/ic/commit/eb7f3dc5c) ~~Boundary Nodes: improve nginx performance~~
* ~~author: Kami Popi |~~ [26f30f055](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height
* ~~author: Leo Eich |~~ [b4673936a](https://github.com/dfinity/ic/commit/b4673936a) Consensus(ecdsa): Make key\\_unmasked\\_ref in PreSignatureQuadrupleRef required
* ~~author: Leo Eich |~~ [b733f7043](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* ~~author: Leo Eich | [6a4d8962c](https://github.com/dfinity/ic/commit/6a4d8962c) Consensus(ecdsa): Make masked kappa config optional~~
* ~~author: Leo Eich |~~ [e76c5a374](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares
* ~~author: Leo Eich |~~ [2d63da24c](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation
""",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        )

    assert publish_client.ensure_published.call_count == 0  # pylint: disable=no-member
