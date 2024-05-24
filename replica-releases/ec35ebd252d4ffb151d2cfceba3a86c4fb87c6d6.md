Release Notes for [**release-2024-05-22\_23-01-base**](https://github.com/dfinity/ic/tree/release-2024-05-22_23-01-base) (ec35ebd252d4ffb151d2cfceba3a86c4fb87c6d6)
===================================================================================================================================================================

Changelog since git revision [5ba1412f9175d987661ae3c0d8dbd1ac3e092b7d](https://dashboard.internetcomputer.org/release/5ba1412f9175d987661ae3c0d8dbd1ac3e092b7d)

Features:
---------

* [`715c74e4f`](https://github.com/dfinity/ic/commit/715c74e4f) Crypto: remove the legacy TlsHandshake interface
* [`4b6c64f75`](https://github.com/dfinity/ic/commit/4b6c64f75) Execution,Message Routing: Implement all message stats in MessagePool
* [`a5bb2a8ab`](https://github.com/dfinity/ic/commit/a5bb2a8ab) Interface: Enable new storage layer
* [`b8c05ba56`](https://github.com/dfinity/ic/commit/b8c05ba56) Message Routing: Don't use the legacy internal API for establishing TLS connection, instead use the rustls configs
* [`9e2fc26c3`](https://github.com/dfinity/ic/commit/9e2fc26c3) Node: Consolidate rootfs utils #8
* [`9a670a31b`](https://github.com/dfinity/ic/commit/9a670a31b) Node,Consensus: guestos: enable bouncer in API BN
* [`cbf83e6ce`](https://github.com/dfinity/ic/commit/cbf83e6ce) Runtime: Support for wasm64 in native stable64
* [`cd8c75eb3`](https://github.com/dfinity/ic/commit/cd8c75eb3) Execution: Enable canister logging feature flag
* [`c55eee381`](https://github.com/dfinity/ic/commit/c55eee381) Runtime,Execution: Separate ContractViolation errors

Bugfixes:
---------

* [`b81813125`](https://github.com/dfinity/ic/commit/b81813125) Networking: use the rustls client config instead of calling perform\_tls\_client\_handshake
* [`2ec1dbd9a`](https://github.com/dfinity/ic/commit/2ec1dbd9a) Runtime,Execution: Use saturating multiplication to calculate byte transmission cost

-----------------------------


Chores:
-------

* [`141e59113`](https://github.com/dfinity/ic/commit/141e59113) Execution: Implement routing for iDKG messages and extend network topology with signing subnets
* [`dc3af91f8`](https://github.com/dfinity/ic/commit/dc3af91f8) Boundary Nodes,Node(boundary-node): shell script cosmetics
* [`52e0587d5`](https://github.com/dfinity/ic/commit/52e0587d5) Boundary Nodes,Node(boundary-node): Add rate limit for new connections and global connection limit [S3\_UPLOAD]
* [`bd620bdc0`](https://github.com/dfinity/ic/commit/bd620bdc0) Boundary Nodes,Node(boundary-node): cleanup firewall
* [`376ecf7fd`](https://github.com/dfinity/ic/commit/376ecf7fd) Consensus(schnorr): Populate master public key ID in EcdsaKeyTranscript
* [`1b8c37441`](https://github.com/dfinity/ic/commit/1b8c37441) Consensus(ecdsa): Use new payload layouts for all newly created ECDSA payloads
* [`cb5715b32`](https://github.com/dfinity/ic/commit/cb5715b32) Crypto: adjust crypto\_fine\_grained\_verify\_dealing\_private\_duration\_seconds buckets
* [`e60766877`](https://github.com/dfinity/ic/commit/e60766877) Crypto: Modify domain separator logic in threshold signatures
* [`70268d330`](https://github.com/dfinity/ic/commit/70268d330) Execution: Remove data\_certificate from replicated query
* [`13daaafe5`](https://github.com/dfinity/ic/commit/13daaafe5) Interface: truncate replica logs
* [`1da1a0d27`](https://github.com/dfinity/ic/commit/1da1a0d27) Networking: use hyper directly in http endpoint instead of axum server
* [`9fd5bdca0`](https://github.com/dfinity/ic/commit/9fd5bdca0) Node: Organize ic-os/cpp
* [`d1944bf4a`](https://github.com/dfinity/ic/commit/d1944bf4a) Node: Update SetupOS failure message to direct to Node Deployment troubleshooting guide
* [`a2abc42c3`](https://github.com/dfinity/ic/commit/a2abc42c3) Node,Consensus(api-bn): remove ICMP type parameter-problem
* [`335781a96`](https://github.com/dfinity/ic/commit/335781a96) Runtime: Split system api list for validation
* [`36ed9bc99`](https://github.com/dfinity/ic/commit/36ed9bc99) Runtime,Execution: Add instruction limit to the relevant error message

Refactoring:
------------

* [`d0af7abc2`](https://github.com/dfinity/ic/commit/d0af7abc2) Crypto: don't pull tokio-rustls into crypto and remove unused deps from orchestrator
* [`257a8fa77`](https://github.com/dfinity/ic/commit/257a8fa77) Crypto: use rustls instead the re-exported module from tokio-rustls
* [`ba603cf0e`](https://github.com/dfinity/ic/commit/ba603cf0e) Crypto: use the TlsConfig trait instead of TlsHandshake
* [`4ca4593c1`](https://github.com/dfinity/ic/commit/4ca4593c1) Execution,Message Routing: Wrap Callbacks in an Arc
* [`1579e0d85`](https://github.com/dfinity/ic/commit/1579e0d85) Execution,Runtime: Clean up process\_stopping\_canisters
* [`35661b175`](https://github.com/dfinity/ic/commit/35661b175) Networking,Message Routing: (re)move unneed rustls deps

Tests:
------

* [`8a3a779ec`](https://github.com/dfinity/ic/commit/8a3a779ec) Crypto: add different message sizes in basic sig bench
* [`01ca07876`](https://github.com/dfinity/ic/commit/01ca07876) Crypto: improve parameters in IDKG reshare\_key\_transcript tests
* [`24adb86c3`](https://github.com/dfinity/ic/commit/24adb86c3) Execution: Refactor system API availability tests
* [`6b13e7b88`](https://github.com/dfinity/ic/commit/6b13e7b88) Execution,Consensus(consensus): Add a query stats log message
* [`0e1b47806`](https://github.com/dfinity/ic/commit/0e1b47806) Interface: Add round trip encoding test for StopCanisterContext
* [`d7a771575`](https://github.com/dfinity/ic/commit/d7a771575) Networking(http-endpoint): Parameterise integration tests with rstest crate
* [`db017130c`](https://github.com/dfinity/ic/commit/db017130c) Networking(http-endpoint): Remove agent-rs dependency for integration tests.
* [`d522089b7`](https://github.com/dfinity/ic/commit/d522089b7) Networking(http-endpoint): Remove agent-rs dependency for read\_state tests
* [`497e403b4`](https://github.com/dfinity/ic/commit/497e403b4) Networking(http-endpoint): Remove agent-rs dependency for Query tests
* [`6fe66d4f9`](https://github.com/dfinity/ic/commit/6fe66d4f9) Networking(http-endpoint): Remove agent-rs dependency for update call tests
* [`2ed2eb131`](https://github.com/dfinity/ic/commit/2ed2eb131) Node: Remove config validation test

Other changes:
--------------

* [`8d09e42d8`](https://github.com/dfinity/ic/commit/8d09e42d8) Execution,Interface,Consensus: Remove old reject code field
* [`d32e03dde`](https://github.com/dfinity/ic/commit/d32e03dde) Node: Updating container base images refs [2024-05-16-0816]
* [`4e221b549`](https://github.com/dfinity/ic/commit/4e221b549) Node,Consensus: () rate and connection limits in nftables for API BN
