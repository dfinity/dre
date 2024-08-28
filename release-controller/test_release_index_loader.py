from release_index_loader import GitReleaseLoader
from publish_notes import post_process_release_notes
import pathlib


def test_remove_excluded_changes(mocker):
    processed = post_process_release_notes(
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

Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (b0ade55f7e8999e2842fe3f49df163ba224b71a2)
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
"""
    )

    loader = GitReleaseLoader("https://github.com/dfinity/dre.git")
    mocker.patch.object(loader, "changelog", return_value=processed)
    assert (
        # we're using existing proposal here but mocking the input/output. TODO: change to use the real proposal instead of mocking
        loader.proposal_summary("b0ade55f7e8999e2842fe3f49df163ba224b71a2")
        == """\
Review checklist
================

Please cross-out your team once you finished the review


Release Notes for [**rc--2024-02-21\\_23-01**](https://github.com/dfinity/ic/tree/rc--2024-02-21_23-01) (b0ade55f7e8999e2842fe3f49df163ba224b71a2)
=================================================================================================================================================

Changelog since git revision [8d4b6898d878fa3db4028b316b78b469ed29f293](https://dashboard.internetcomputer.org/release/8d4b6898d878fa3db4028b316b78b469ed29f293)

Features:
---------

* [`26f30f055`](https://github.com/dfinity/ic/commit/26f30f055) Consensus: Purge non-finalized blocks and notarizations below the finalized height
* [`b733f7043`](https://github.com/dfinity/ic/commit/b733f7043) Consensus(ecdsa): Extend Quadruple state machine in preparation for random unmasked kappa
* [`e76c5a374`](https://github.com/dfinity/ic/commit/e76c5a374) Consensus(ecdsa): Stop relaying tECDSA signature shares
* [`2d63da24c`](https://github.com/dfinity/ic/commit/2d63da24c) Consensus(ecdsa): Add optional kappa\\_unmasked config to QuadrupleInCreation

Full list of changes (including the ones that are not relevant to GuestOS) can be found on [GitHub](https://github.com/dfinity/dre/blob/c710c73c24e83fc62c848540f63a4eb351862c99/replica-releases/b0ade55f7e8999e2842fe3f49df163ba224b71a2.md).

# IC-OS Verification

To build and verify the IC-OS disk image, run:

```
# From https://github.com/dfinity/ic#verifying-releases
sudo apt-get install -y curl && curl --proto \'=https\' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/b0ade55f7e8999e2842fe3f49df163ba224b71a2/gitlab-ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c b0ade55f7e8999e2842fe3f49df163ba224b71a2
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image, must be identical, and must match the SHA256 from the payload of the NNS proposal.
"""
    )
