Release Notes for [**release-2025-06-05\_03-24-base**](https://github.com/dfinity/ic/tree/release-2025-06-05_03-24-base) (8f1ef8ce78361adbc09aea4c2f0bce701c9ddb4d)
===================================================================================================================================================================

This release is based on changes since [release-2025-05-30\_03-21-base](https://dashboard.internetcomputer.org/release/ed3650da85f390130dedf55806da9337d796b799) (ed3650da85f390130dedf55806da9337d796b799).

Please note that some commits may be excluded from this release if they're not relevant, or not modifying the GuestOS image. Additionally, descriptions of some changes might have been slightly modified to fit the release notes format.

To see a full list of commits added since last release, compare the revisions on [GitHub](https://github.com/dfinity/ic/compare/release-2025-05-30_03-21-base...release-2025-06-05_03-24-base).

Bugfixes:
---------

* [`190c44caa`](https://github.com/dfinity/ic/commit/190c44caa) Execution,Interface: Sanitize input to canister snapshot data download before charging cycles ([#5344](https://github.com/dfinity/ic/pull/5344))
* [`53c1713ec`](https://github.com/dfinity/ic/commit/53c1713ec) Execution,Interface: return deallocated bytes to subnet available memory ([#5362](https://github.com/dfinity/ic/pull/5362))
* [`6cd54499b`](https://github.com/dfinity/ic/commit/6cd54499b) Interface: error message for OutOfMemory ([#5358](https://github.com/dfinity/ic/pull/5358))
* [`ce1287d35`](https://github.com/dfinity/ic/commit/ce1287d35) Interface,Node: Change behavior when the GuestOS VM config already exists to make it consistent with the previous behavior ([#5331](https://github.com/dfinity/ic/pull/5331))
* [`27e24aefc`](https://github.com/dfinity/ic/commit/27e24aefc) Interface,Node: Add consistent defaults for HostOSSettings ([#5350](https://github.com/dfinity/ic/pull/5350))

Chores:
-------

* [`8f1ef8ce7`](https://github.com/dfinity/ic/commit/8f1ef8ce7) Consensus,Interface: Distinguish transient VetKD errors where NiDkgTranscript wasn't loaded yet ([#5404](https://github.com/dfinity/ic/pull/5404))
* [`029ebf5c4`](https://github.com/dfinity/ic/commit/029ebf5c4) Interface: Upgrade canbench to 0.15.0 ([#5356](https://github.com/dfinity/ic/pull/5356))
* [`c45812358`](https://github.com/dfinity/ic/commit/c45812358) Interface: downgrade to rust 1.85.1 ([#5194](https://github.com/dfinity/ic/pull/5194))
* [`24e8ba331`](https://github.com/dfinity/ic/commit/24e8ba331) Owners: bump ic-management-canister-types to 0.3.1 ([#5408](https://github.com/dfinity/ic/pull/5408))
* [`b0f19d5dc`](https://github.com/dfinity/ic/commit/b0f19d5dc) Node: Remove local-vm tool from bazel ([#5373](https://github.com/dfinity/ic/pull/5373))

Refactoring:
------------

* [`c2f9448f4`](https://github.com/dfinity/ic/commit/c2f9448f4) Consensus,Interface(registration): unify and enhance node registration logs ([#5376](https://github.com/dfinity/ic/pull/5376))

-------------------------------------------

## Excluded Changes

### Changed files are excluded by file path filter
* [`7c9a1abf3`](https://github.com/dfinity/ic/commit/7c9a1abf3) Interface(nns): Support disbursing maturity to an AccountIdentifier ([#5351](https://github.com/dfinity/ic/pull/5351))
* [`8b78380f8`](https://github.com/dfinity/ic/commit/8b78380f8) Interface: Use mint\_cycles128 in CMC ([#5317](https://github.com/dfinity/ic/pull/5317))
* [`e2def7182`](https://github.com/dfinity/ic/commit/e2def7182) Interface(PocketIC): deterministic registry and validation ([#5379](https://github.com/dfinity/ic/pull/5379))

### Not modifying GuestOS
* [`250daf4dd`](https://github.com/dfinity/ic/commit/250daf4dd) Interface(nns): Enable disburse maturity through neuron management proposals ([#5157](https://github.com/dfinity/ic/pull/5157))
* [`6360f4b45`](https://github.com/dfinity/ic/commit/6360f4b45) Interface(ICP-Ledger): add approve and transfer from benchmarks ([#5360](https://github.com/dfinity/ic/pull/5360))
* [`0233b7fa1`](https://github.com/dfinity/ic/commit/0233b7fa1) Interface(icp-ledger): add basic canbench benchmarks ([#5261](https://github.com/dfinity/ic/pull/5261))
* [`97abe8361`](https://github.com/dfinity/ic/commit/97abe8361) Owners: Mirror new 24.04 ubuntu image ([#5378](https://github.com/dfinity/ic/pull/5378))
* [`7de799932`](https://github.com/dfinity/ic/commit/7de799932) Interface(boundary): prometheus scraping playnet url ([#5377](https://github.com/dfinity/ic/pull/5377))
* [`7058c2ae9`](https://github.com/dfinity/ic/commit/7058c2ae9) Owners: not using an action to setup gh cli ([#5393](https://github.com/dfinity/ic/pull/5393))
* [`b70fab27e`](https://github.com/dfinity/ic/commit/b70fab27e) Owners(fuzzing): Fix path for GCP key ([#5402](https://github.com/dfinity/ic/pull/5402))
* [`16b14f87c`](https://github.com/dfinity/ic/commit/16b14f87c) Owners(IDX): only upload legacy artifacts for x86 linux ([#5396](https://github.com/dfinity/ic/pull/5396))
* [`230b09e8d`](https://github.com/dfinity/ic/commit/230b09e8d) Owners(IDX): run versions workflow every 2 hours ([#5392](https://github.com/dfinity/ic/pull/5392))
* [`ef9696248`](https://github.com/dfinity/ic/commit/ef9696248) Consensus,Interface(boundary): remove legacy boundary node from socks proxy test ([#5390](https://github.com/dfinity/ic/pull/5390))
* [`e7821abde`](https://github.com/dfinity/ic/commit/e7821abde) Execution,Interface: Add section about error variants to contribution guide. ([#5343](https://github.com/dfinity/ic/pull/5343))
* [`dde0d59ea`](https://github.com/dfinity/ic/commit/dde0d59ea) Interface(nns): Delete //rs/nns/governance:scale\_bench ([#5397](https://github.com/dfinity/ic/pull/5397))
* [`a2506113c`](https://github.com/dfinity/ic/commit/a2506113c) Interface(XC): delete ckBTC staging canisters ([#5413](https://github.com/dfinity/ic/pull/5413))
* [`930a88fec`](https://github.com/dfinity/ic/commit/930a88fec) Interface: PocketIC server v9.0.3 and PocketIC library v9.0.2 ([#5341](https://github.com/dfinity/ic/pull/5341))
* [`ec14be384`](https://github.com/dfinity/ic/commit/ec14be384) Interface: move PocketIC HTTP gateway tests into a separate target ([#5406](https://github.com/dfinity/ic/pull/5406))
* [`1fd73ecd5`](https://github.com/dfinity/ic/commit/1fd73ecd5) Interface: remove BN mentions ([#5403](https://github.com/dfinity/ic/pull/5403))
* [`d3649d857`](https://github.com/dfinity/ic/commit/d3649d857) Interface(nns): update release notes ([#5368](https://github.com/dfinity/ic/pull/5368))
* [`4ce533312`](https://github.com/dfinity/ic/commit/4ce533312) Interface(PocketIC): comprehensible error for clearly invalid subnet state directory ([#5380](https://github.com/dfinity/ic/pull/5380))
* [`d6209cf64`](https://github.com/dfinity/ic/commit/d6209cf64) Interface: stabilize the highly flaky nns\_dapp\_test ([#5372](https://github.com/dfinity/ic/pull/5372))
* [`e38941af5`](https://github.com/dfinity/ic/commit/e38941af5) Interface: remove ic\_mainnet\_nns\_recovery ([#5307](https://github.com/dfinity/ic/pull/5307))
* [`1c41270c4`](https://github.com/dfinity/ic/commit/1c41270c4) Interface(boundary): ic-gateway uvm ([#5303](https://github.com/dfinity/ic/pull/5303))
* [`ff7f57501`](https://github.com/dfinity/ic/commit/ff7f57501) Interface(ckbtc/cketh): add upgrade proposals for BTC Checker and ckETH minter canisters ([#5357](https://github.com/dfinity/ic/pull/5357))
* [`dd2190104`](https://github.com/dfinity/ic/commit/dd2190104) Interface,Message Routing: use ic-cdk 0.18.0-alpha.2 ([#5363](https://github.com/dfinity/ic/pull/5363))
* [`1fba003fe`](https://github.com/dfinity/ic/commit/1fba003fe) Owners: Update CODEOWNERS for protobuf generated files ([#5398](https://github.com/dfinity/ic/pull/5398))
* [`a91e3349e`](https://github.com/dfinity/ic/commit/a91e3349e) Owners: Update Mainnet IC revisions file ([#5388](https://github.com/dfinity/ic/pull/5388))
* [`527f4c26c`](https://github.com/dfinity/ic/commit/527f4c26c) Owners: Update code owners for lru cache ([#5375](https://github.com/dfinity/ic/pull/5375))
* [`d025543ad`](https://github.com/dfinity/ic/commit/d025543ad) Owners: update codeowners for /rs/interfaces/src/execution\_environment/ ([#5371](https://github.com/dfinity/ic/pull/5371))
* [`084991f40`](https://github.com/dfinity/ic/commit/084991f40) Interface(nns): Move Governance API->Internal type conversion into Governance::new ([#5400](https://github.com/dfinity/ic/pull/5400))
* [`4ac814456`](https://github.com/dfinity/ic/commit/4ac814456) Interface(cketh): remove SingleCallError and HttpOutcallError, remove redundant reduction in eth\_get\_finalized\_transaction\_count and eth\_get\_block\_by\_number ([#5268](https://github.com/dfinity/ic/pull/5268))
* [`78a352c38`](https://github.com/dfinity/ic/commit/78a352c38) Node: Simplify OS variant conditionals ([#5381](https://github.com/dfinity/ic/pull/5381))
* [`d5e7656c4`](https://github.com/dfinity/ic/commit/d5e7656c4) Owners: "chore: Update Mainnet IC revisions file" ([#5394](https://github.com/dfinity/ic/pull/5394))

### Scope of the change (registry) is not related to the artifact
* [`d7fcb9aa4`](https://github.com/dfinity/ic/commit/d7fcb9aa4) Interface(registry): Add max\_rewardable\_nodes to NodeOperatorRecord ([#5267](https://github.com/dfinity/ic/pull/5267))

### Scope of the change (sns) is not related to the artifact
* [`c3ae9cc72`](https://github.com/dfinity/ic/commit/c3ae9cc72) Interface(sns): inactive status of SNS ([#5079](https://github.com/dfinity/ic/pull/5079))
