Release Notes for [**release-2024-07-30\_01-30-base**](https://github.com/dfinity/ic/tree/release-2024-07-30_01-30-base) (2bdfdc54ccc7ef27dd7b4f37aaea172198dce6ab)
===================================================================================================================================================================

This release is based on [release-2024-07-25\_21-03-base](https://dashboard.internetcomputer.org/release/2c0b76cfc7e596d5c4304cff5222a2619294c8c1) (2c0b76cfc7e596d5c4304cff5222a2619294c8c1).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, some desriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2024-07-25_21-03-base...release-2024-07-30_01-30-base).

Features:
---------

* [`a1fc94c52`](https://github.com/dfinity/ic/commit/a1fc94c52) pocket-ic(PocketIC): new endpoint to list HTTP gateways (#636)
* [`3f8baa2f2`](https://github.com/dfinity/ic/commit/3f8baa2f2) pocket-ic(PocketIC): specify IP address of PocketIC server and HTTP gateway (#634)

Bugfixes:
---------

* [`2bdfdc54c`](https://github.com/dfinity/ic/commit/2bdfdc54c) Consensus: Use correct signer id in make\_next\_block\_with\_rank (#644)
* [`9fc5fc83f`](https://github.com/dfinity/ic/commit/9fc5fc83f) Interface: Update computation of effective canister id for FetchCanisterLogs (#540)
* [`dd2fe6092`](https://github.com/dfinity/ic/commit/dd2fe6092) pocket-ic(PocketIC): block until HTTP handler starts (#637)

Chores:
-------

* [`dbfbeceea`](https://github.com/dfinity/ic/commit/dbfbeceea) Boundary Nodes,Networking: bump jemallocator v0.3 to tikv-jemallocator v0.5 (#654)
* [`668fbe08f`](https://github.com/dfinity/ic/commit/668fbe08f) Consensus,Execution,Runtime: Rename ECDSA metrics (#535)
* [`ec01b3735`](https://github.com/dfinity/ic/commit/ec01b3735) Consensus,Interface: add tools-pkg (#584)
* [`4bebd6f6a`](https://github.com/dfinity/ic/commit/4bebd6f6a) Execution,Runtime: Add Wasm memory threshold field to canister settings (#475)
* [`3e9785f87`](https://github.com/dfinity/ic/commit/3e9785f87) Execution,Runtime: Rename fees\_and\_limits to icp\_config (#638)
* [`0527e6f50`](https://github.com/dfinity/ic/commit/0527e6f50) Message Routing: Use a single sentence for error messages in IngressInductionError (#648)
* [`173d06185`](https://github.com/dfinity/ic/commit/173d06185) IDX,Node: build and strip IC-OS tools iff we build the VMs (#609)

Refactoring:
------------

* [`9ff9f96b0`](https://github.com/dfinity/ic/commit/9ff9f96b0) Crypto: remove CspTlsHandshakeSignerProvider (#627)
* [`1909c13a8`](https://github.com/dfinity/ic/commit/1909c13a8) Crypto: remove CspPublicKeyStore (#625)
* [`50857b09e`](https://github.com/dfinity/ic/commit/50857b09e) Message Routing: Move IngressInductionError outside of replicated state (#618)

Tests:
------

* [`402a3a6f3`](https://github.com/dfinity/ic/commit/402a3a6f3) Consensus,IDX(consensus): Push consensus performance test results to Elasticsearch (#646)
* [`98797bd8f`](https://github.com/dfinity/ic/commit/98797bd8f) Consensus,IDX(consensus): extract more utility functions into tests/consensus/utils (#639)
* [`e006612ff`](https://github.com/dfinity/ic/commit/e006612ff) Consensus,IDX(consensus): Inline more consensus tests (#632)
* [`b486455bd`](https://github.com/dfinity/ic/commit/b486455bd) Consensus,IDX: Inline remaining tECDSA tests (#619)
* [`32bc2c260`](https://github.com/dfinity/ic/commit/32bc2c260) Message Routing,IDX: Use mainnet binaries for the queues compatibility test (#419)
