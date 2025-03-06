  

|ICRC|Title|Author|Discussions|Status|Type|Category|Created|
|---|---|---|---|---|---|---|---|
|37|Approval Functionality for the ICRC-7 Non-Fungible Token (NFT) Standard|Ben Zhai (@benjizhai), Austin Fatheree (@skilesare), Dieter Sommer (@dietersommer), Thomas (@sea-snake), Moritz Fuller (@letmejustputthishere), Matthew Harmon|[https://github.com/dfinity/ICRC/issues/37](https://github.com/dfinity/ICRC/issues/37)|Draft|Standards Track||2023-11-22|

# ICRC-37: Approval Support for the Minimal Non-Fungible Token (NFT) Standard

This document specifies approval support for the [ICRC-7 minimal NFT standard for the Internet Computer](https://github.com/dfinity/ICRC/ICRCs/ICRC-7/ICRC-7.md). It defines all the methods required for realizing approval semantics for an NFT token ledger, i.e., creating approvals, revoking approvals, querying approval information, and making transfers based on approvals. The scope of ICRC-37 has been part of ICRC-7 originally, however, the NFT Working Group has decided to split it out into a separate standard for the following reasons:

- ICRC-7 and ICRC-37 are much shorter and hence easier to navigate on their own due to their respective foci;
- Ledgers that do not want to implement approval and transfer from semantics do not need to provide dummy implementations of the corresponding methods that fail by default.

This standard extends the ICRC-7 NFT standard and is intended to be implemented by token ledgers that implement ICRC-7. An ICRC-7 ledger may implement ICRC-37 in case it intends to offer approve and transfer from semantics. Principles put forth in ICRC-7 apply to ICRC-37 as well, e.g., the design of the update and query API.

> The keywords "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

**Canisters implementing the `ICRC-37` standard MUST implement all the functions in the `ICRC-7` interface**.

**Canisters implementing the `ICRC-37` standard MUST include `ICRC-37` in the list returned by the `icrc7_supported_standards` method**.

## What does the standard imply?

ICRC-37 is a contract between organizations participating in the Ledger & Tokenization working group and the community. If the standard is accepted, we agree to

1. Provide a production-ready implementation of the standard.
2. Support the standard in our existing and future products.
3. Build tools to interact with standard-compliant implementations.
4. Promote the standard.
5. Design extensions to the standard to simplify and scale payment flows.

Accepting the ICRC-37 standard does not imply that all other standards should be considered obsolete. Everyone is free to experiment with new designs, application interfaces, and products.

The main goal of ICRC-37 is to provide a solid common ground for interoperability.

## Specification

The specification of ICRC-37 is too big to fit into a motion proposal. The full ICRC-37 specification in the version to be voted on can be found at [https://github.com/dfinity/ICRC/tree/424402830cbcc2def46112556d13ac5c5ab41c0c/ICRCs/ICRC-37](https://github.com/dfinity/ICRC/tree/424402830cbcc2def46112556d13ac5c5ab41c0c/ICRCs/ICRC-37).