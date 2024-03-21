import pytest
from github import Github
from publish_notes import PublishNotesClient


def test_publish_if_ready__ready(mocker):
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
* ~~@node-team~~
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

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

    publish_client.ensure_published.assert_called_once_with(
        version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        changelog="""\
Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Features:
---------

* [`26f30f055`](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height
* [`b733f7043`](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* [`e76c5a374`](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares
* [`2d63da24c`](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation
""",
    )


def test_publish_if_ready__not_ready(mocker):
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
* @team-execution
* ~~@team-messaging~~
* ~~@team-networking~~
* ~~@node-team~~
* ~~@team-runtime~~

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

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

    assert publish_client.ensure_published.call_count == 0
