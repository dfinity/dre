import pathlib
import tempfile

from git_repo import GitRepo
from release_index_loader import GitReleaseLoader


def test_remove_excluded_changes():
    with tempfile.TemporaryDirectory() as d:
        loader = GitReleaseLoader(
            GitRepo(
                "https://github.com/dfinity/dre.git",
                repo_cache_dir=pathlib.Path(d),
            )
        )
        assert (
            loader.proposal_summary("35bfcadd0f2a474057e42393917b8b3ac269627a")
            == """\
Release Notes for [**release\\-2024\\-08\\-29\\_01\\-30\\-base**](https://github.com/dfinity/ic/tree/release-2024-08-29_01-30-base) (35bfcadd0f2a474057e42393917b8b3ac269627a)
========================================================================================================================================================================

This release is based on changes since [release\\-2024\\-08\\-21\\_15\\-36\\-base](https://dashboard.internetcomputer.org/release/b0ade55f7e8999e2842fe3f49df163ba224b71a2) (b0ade55f7e8999e2842fe3f49df163ba224b71a2\\).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-08-21_15-36-base...release-2024-08-29_01-30-base).

This release diverges from the latest release. Merge base is [7b3981ca032bd5d3c6be349bace7ad2da799baaa](https://github.com/dfinity/ic/tree/7b3981ca032bd5d3c6be349bace7ad2da799baaa). Changes [were removed](https://github.com/dfinity/ic/compare/release-2024-08-29_01-30-base...release-2024-08-21_15-36-base) from this release.

Features:
---------

* [`d37a6a16b`](https://github.com/dfinity/ic/commit/d37a6a16b) Consensus,Interface(consensus): Make validator create equivocation proofs and ignore equivocating blocks ([\\#852](https://github.com/dfinity/ic/pull/852))
* [`35bfcadd0`](https://github.com/dfinity/ic/commit/35bfcadd0) Execution,Interface: Set the default Wasm memory limit ([\\#1040](https://github.com/dfinity/ic/pull/1040))
* [`a63138ab5`](https://github.com/dfinity/ic/commit/a63138ab5) Execution,Interface,Message Routing: Check SystemState invariants on checkpoint loading ([\\#1165](https://github.com/dfinity/ic/pull/1165))
* [`d436a526d`](https://github.com/dfinity/ic/commit/d436a526d) Interface(ic\\-admin): Print hashes rather than entire blobs when submitting InstallCode proposals ([\\#1093](https://github.com/dfinity/ic/pull/1093))
* [`99c86c9e0`](https://github.com/dfinity/ic/commit/99c86c9e0) Interface,Message Routing: Raise a critical error if ReplicatedState gets altered during checkpointing ([\\#1079](https://github.com/dfinity/ic/pull/1079))
* [`60432db15`](https://github.com/dfinity/ic/commit/60432db15) Interface,Networking: Enable synchronous endpoint for snjp subnet ([\\#1194](https://github.com/dfinity/ic/pull/1194))
* [`cd153b5b7`](https://github.com/dfinity/ic/commit/cd153b5b7) Interface,Networking: Create asynchronous handler for v3 call endpoint by default ([\\#1151](https://github.com/dfinity/ic/pull/1151))

Bugfixes:
---------

* [`124957f40`](https://github.com/dfinity/ic/commit/124957f40) Consensus,Interface,Networking(IDX): remove links to unknown rustdoc refs ([\\#1145](https://github.com/dfinity/ic/pull/1145))

Chores:
-------

* [`1c02246bb`](https://github.com/dfinity/ic/commit/1c02246bb) Consensus,Interface(consensus): add more checks to the dkg payload validator ([\\#1078](https://github.com/dfinity/ic/pull/1078))
* [`232964087`](https://github.com/dfinity/ic/commit/232964087) Consensus,Interface(consensus): small clean\\-ups in Ingress Pool ([\\#1066](https://github.com/dfinity/ic/pull/1066))
* [`a3e2dac1b`](https://github.com/dfinity/ic/commit/a3e2dac1b) Crypto,Interface: upgrade deps and use workspace version ([\\#1077](https://github.com/dfinity/ic/pull/1077))
* [`3c66cc522`](https://github.com/dfinity/ic/commit/3c66cc522) Execution,Interface: Remove obsolete LogVisibility v1 type ([\\#1139](https://github.com/dfinity/ic/pull/1139))
* [`7dbc6d425`](https://github.com/dfinity/ic/commit/7dbc6d425) Execution,Interface: Minor changes in canister snapshotting ([\\#1021](https://github.com/dfinity/ic/pull/1021))
* [`9a84997c0`](https://github.com/dfinity/ic/commit/9a84997c0) Execution,Interface: Reserve cycles just for the increase of canister snapshot size ([\\#960](https://github.com/dfinity/ic/pull/960))
* [`46e1372d2`](https://github.com/dfinity/ic/commit/46e1372d2) Execution,Interface(ic): Unify wasm\\-tools dependency versions ([\\#1125](https://github.com/dfinity/ic/pull/1125))
* [`0331e769f`](https://github.com/dfinity/ic/commit/0331e769f) Execution,Interface,Message Routing: Remove logic for deprecated reject signals. ([\\#1037](https://github.com/dfinity/ic/pull/1037))
* [`a4fefd9c7`](https://github.com/dfinity/ic/commit/a4fefd9c7) Interface: upgrade hyper in test driver ([\\#1106](https://github.com/dfinity/ic/pull/1106))
* [`62c2ed16a`](https://github.com/dfinity/ic/commit/62c2ed16a) Interface: Some typos in ValidateEq. ([\\#1177](https://github.com/dfinity/ic/pull/1177))
* [`d71e09e83`](https://github.com/dfinity/ic/commit/d71e09e83) Interface: add decoding quota to http\\_request in SNS and ICRC1 canisters ([\\#1101](https://github.com/dfinity/ic/pull/1101))
* [`4e5d6322b`](https://github.com/dfinity/ic/commit/4e5d6322b) Interface: add decoding quota to http\\_request in NNS canisters ([\\#1060](https://github.com/dfinity/ic/pull/1060))
* [`c6e64a7e3`](https://github.com/dfinity/ic/commit/c6e64a7e3) Interface(crypto): Rename ic\\_crypto\\_ecdsa\\_secp256k1 crate ([\\#999](https://github.com/dfinity/ic/pull/999))
* [`1e38e12c7`](https://github.com/dfinity/ic/commit/1e38e12c7) Interface,Message Routing: simplify further the hyper/xnet code ([\\#1107](https://github.com/dfinity/ic/pull/1107))
* [`07f4e545b`](https://github.com/dfinity/ic/commit/07f4e545b) Interface,Message Routing: upgrade hyper in xnet ([\\#758](https://github.com/dfinity/ic/pull/758))
* [`1e96fa09e`](https://github.com/dfinity/ic/commit/1e96fa09e) Interface,Message Routing: Bump certification version to v19 ([\\#1035](https://github.com/dfinity/ic/pull/1035))
* [`cb388ee94`](https://github.com/dfinity/ic/commit/cb388ee94) Interface,Message Routing,Networking: delete unused threads backtrace debug http endpoint ([\\#947](https://github.com/dfinity/ic/pull/947))
* [`77405f50d`](https://github.com/dfinity/ic/commit/77405f50d) Interface,Networking: crate the https outcalls adapter only by passing a config ([\\#1076](https://github.com/dfinity/ic/pull/1076))
* [`a5f59cf3f`](https://github.com/dfinity/ic/commit/a5f59cf3f) Interface,Networking: update ingress watcher cancellation log level ([\\#1133](https://github.com/dfinity/ic/pull/1133))
* [`a5595edf2`](https://github.com/dfinity/ic/commit/a5595edf2) Interface,Networking: https outcalls hyper upgrade ([\\#1017](https://github.com/dfinity/ic/pull/1017))
* [`3c9aaf594`](https://github.com/dfinity/ic/commit/3c9aaf594) Interface,Networking(http\\-handler): Track execution \\+ certification time for all ingress messages ([\\#1022](https://github.com/dfinity/ic/pull/1022))
* [`a8b1a1912`](https://github.com/dfinity/ic/commit/a8b1a1912) Interface,Networking(http\\-metrics): Increase number of buckets for certification time of messages ([\\#1026](https://github.com/dfinity/ic/pull/1026))
* [`45e3038eb`](https://github.com/dfinity/ic/commit/45e3038eb) Interface,Networking: remove the advert PB ([\\#1036](https://github.com/dfinity/ic/pull/1036))
* [`6fd620f4a`](https://github.com/dfinity/ic/commit/6fd620f4a) Node: Move the setup/teardown of temporary build directories to a process wrapper ([\\#1142](https://github.com/dfinity/ic/pull/1142))
* [`8a5c77e48`](https://github.com/dfinity/ic/commit/8a5c77e48) Node: Update Base Image Refs \\[2024\\-08\\-22\\-0808] ([\\#1059](https://github.com/dfinity/ic/pull/1059))
* [`3d83a4d2e`](https://github.com/dfinity/ic/commit/3d83a4d2e) Owners(reprocheck): use zst in reprocheck for all images ([\\#1136](https://github.com/dfinity/ic/pull/1136))

Refactoring:
------------

* [`c890f067f`](https://github.com/dfinity/ic/commit/c890f067f) Consensus,Interface: clarify the priority fn semantics ([\\#1042](https://github.com/dfinity/ic/pull/1042))
* [`8520bf65d`](https://github.com/dfinity/ic/commit/8520bf65d) Crypto,Interface(crypto): remove CspSigVerifier ([\\#653](https://github.com/dfinity/ic/pull/653))
* [`5c02bc65d`](https://github.com/dfinity/ic/commit/5c02bc65d) Interface(nervous\\_system): Add a Request trait to simplify interacting with our canisters ([\\#1091](https://github.com/dfinity/ic/pull/1091))
* [`5b7ebe284`](https://github.com/dfinity/ic/commit/5b7ebe284) Interface(nns): Put TimeWarp in API crate to remove dependency ([\\#1122](https://github.com/dfinity/ic/pull/1122))
* [`211fa36d1`](https://github.com/dfinity/ic/commit/211fa36d1) Interface,Networking(http\\-handler): Return a concrete type in the call v2 handler ([\\#1049](https://github.com/dfinity/ic/pull/1049))

----------


Documentation:
--------------

* [`2ac785b95`](https://github.com/dfinity/ic/commit/2ac785b95) Interface,Networking: update the quic transport docs and use thiserror ([\\#1192](https://github.com/dfinity/ic/pull/1192))
* [`7809eee25`](https://github.com/dfinity/ic/commit/7809eee25) Node: update configuration documentation ([\\#1089](https://github.com/dfinity/ic/pull/1089))

Other changes:
--------------

* [`d0719bf22`](https://github.com/dfinity/ic/commit/d0719bf22) Interface,Message Routing: last two xnet commits due to flakiness ([\\#1169](https://github.com/dfinity/ic/pull/1169))

-------------------------------------------


Full list of changes (including the ones that are not relevant to GuestOS) can be found on [GitHub](https://github.com/dfinity/dre/blob/00094e1dc3fc52e00cb01585a633fca25d971fe9/replica-releases/35bfcadd0f2a474057e42393917b8b3ac269627a.md).

# IC-OS Verification

To build and verify the IC-OS disk image, run:

```
# From https://github.com/dfinity/ic#verifying-releases
sudo apt-get install -y curl && curl --proto \'=https\' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/35bfcadd0f2a474057e42393917b8b3ac269627a/ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c 35bfcadd0f2a474057e42393917b8b3ac269627a
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image, must be identical, and must match the SHA256 from the payload of the NNS proposal.
"""
        )
