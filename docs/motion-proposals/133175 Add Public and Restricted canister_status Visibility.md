# Proposal to add Public and Restricted canister_status Visibility

This proposal is submitted by the [CycleOps team](https://forum.dfinity.org/t/meet-cycleops-proactive-automated-no-code-canister-management-for-the-internet-computer/20969). CycleOps is proactive, automated no code canister management for the Internet Computer.

Additional Credits: Thanks to Fulco Taen, the original proposer of a [public canister_status](https://forum.dfinity.org/t/nns-proposal-make-canister-status-public-to-anyone/15775) in October 2022, and thanks to Dominic Woerner for helping push this proposal forward and to Dimitris Sarlis for revewing this proposal.

## Summary

There is a function on the Internet Computer management canister called [canister_status](https://internetcomputer.org/docs/current/references/ic-interface-spec/#ic-canister_status) which returns detailed metric information about the canister, including but not limited to its controllers, wasm hash, cycle balance, and memory utilization. Currently this can only be called by the controllers (owners) of a canister.

This goal of this proposal is to allow the controllers of a canister the voluntary option to make the canister_status API callable by third parties without requiring those parties be a controller of a canister, and to similarly provide the ability for those owners to revoke a canister’s public `canister_status` endpoint to make this data private again at any time.

## Real World Uses Cases

- **Public metrics for public canisters** - SNS and NNS canisters can publicly expose metrics without needing to provide a public API.
- **Protocol verified Metrics** - user created canisters such as NFID vaults, OpenChat user canisters, or even Bob miner canisters can expose metrics that are verified by the protocol instead of via a 3rd party API.
- **Easier canister monitoring** - Provide an alternative option for developers who wish to set up canister monitoring services without requiring them to integrate with a monitoring blackhole.

## Background

In late 2022, an Internet Computer developer [proposed several options for making a canister’s status public](https://forum.dfinity.org/t/nns-proposal-make-canister-status-public-to-anyone/15775). Many of the responding developers in the thread favored option D, “Make canister_status something that the controller can choose to expose or not with a flag which is set to private by default”.

In March 2024, a DFINITY team member opened up a [poll of different options for making a canister’s status public](https://forum.dfinity.org/t/nns-proposal-make-canister-status-public-to-anyone/15775/59), with the majority of respondents voting for the option to “Allow canister_status to be made public with the understanding this could expose secrets and could be extended to make all code & state of the canister public, i.e, public `canister_status` == public canister.”

All respondents favored adding a route for making canister status public, with none believing that the status quo is sufficient.

This proposal suggests a flexible path forward for publicly and selectively exposing canister metrics to third parties.

## Proposed Mechanism for Adding canister_status Visibility

Given that the Internet Computer protocol already provides canister status metric information but restricts access of it to the controller(s) of a canister, we propose that a `status_visibility` variant property should be added to the `canister_settings` returned by `canister_status`.

We propose that `status_visibility` have a `public` , `controllers`, and third `allowed_viewers` option that would bridge the gap between fully private and fully public canisters by allowing a short list (limit 5 principals) that are able to retrieve the canister status for that canister, without requiring that the status be publicly available, or only available to controllers.

Specifying the `allowed_viewers` variant would restrict `canister_status` access to both the list of `allowed_viewers` and any controllers of the canister.

#### Proposed Interface Change

```
type status_visibility = variant {
  controllers;
  allowed_viewers : opt vec principal;
  public;
};

type definite_canister_settings = record {
  controllers : vec principal;
  compute_allocation : nat;
  memory_allocation : nat;
  freezing_threshold : nat;
  reserved_cycles_limit : nat;
  log_visibility : opt log_visibility;
  status_visibility : opt status_visibility; // new field (similar to log_visibility)
};
```

## What is asked of the community

Review the full developer forum proposal, ask questions, and vote to accept or reject.

Forum proposal (live since June 2024): [https://forum.dfinity.org/t/nns-proposal-add-public-and-restricted-canister-status-visibility/31814](https://forum.dfinity.org/t/nns-proposal-add-public-and-restricted-canister-status-visibility/31814)
