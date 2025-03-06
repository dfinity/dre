  

|ICRC|Title|Author|Discussions|Status|Type|Category|Created|
|---|---|---|---|---|---|---|---|
|7|Minimal Non-Fungible Token (NFT) Standard|Ben Zhai (@benjizhai), Austin Fatheree (@skilesare), Dieter Sommer (@dietersommer), Thomas (@sea-snake), Moritz Fuller (@letmejustputthishere), Matthew Harmon|[https://github.com/dfinity/ICRC/issues/7](https://github.com/dfinity/ICRC/issues/7)|Draft|Standards Track||2023-01-31|

# ICRC-7: Minimal Non-Fungible Token (NFT) Standard

ICRC-7 is the minimal standard for the implementation of Non-Fungible Tokens (NFTs) on the [Internet Computer](https://internetcomputer.org/).

A token ledger implementation following this standard hosts an _NFT collection_ (_collection_), which is a set of NFTs.

ICRC-7 does not handle approval-related operations such as `approve` and `transfer_from` itself. Those operations are specified by ICRC-37 which extends ICRC-7 with approval semantics.

## What does the standard imply?

ICRC-7 is a contract between organizations participating in the Ledger & Tokenization working group and the community. If the standard is accepted, we agree to

1. Provide a production-ready implementation of the standard.
2. Support the standard in our existing and future products.
3. Build tools to interact with standard-compliant implementations.
4. Promote the standard.
5. Design extensions to the standard to simplify and scale payment flows.

Accepting the ICRC-7 standard does not imply that all other standards should be considered obsolete. Everyone is free to experiment with new designs, application interfaces, and products.

The main goal of ICRC-7 is to provide a solid common ground for interoperability.

## Specification

The specification of ICRC-7 is too big to fit into a motion proposal. The full ICRC-7 specification in the version to be voted on can be found at [https://github.com/dfinity/ICRC/tree/424402830cbcc2def46112556d13ac5c5ab41c0c/ICRCs/ICRC-7](https://github.com/dfinity/ICRC/tree/424402830cbcc2def46112556d13ac5c5ab41c0c/ICRCs/ICRC-7).