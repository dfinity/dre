# Scalable Messaging Model

## Background

The IC’s messaging model (as of January 2024) is conceptually an attractive proposition: remote procedure calls (RPCs) with reliable replies and automatic call bookkeeping through call contexts. Unpacking it a bit:

- Canisters interact via RPCs
    - A ⇾ B _request_ (message); followed by B ⇾ A _response_ (message).
    - Every call being handled (Motoko _shared function_; or Rust _update function_) is backed by a call context.
- Best-effort requests
    - With _ordering guarantees_: if canister A makes two calls, **c1** and **c2** (in that order) to canister B, then **c2** will not be delivered before **c1**.
- Guaranteed responses
    - There is _globally exactly one response_ (reply or reject) for every request.
    - This response is _eventually delivered_ to the caller.
- Backpressure mechanism
    - Canister A may only have a limited number of outstanding calls to canister B.

## Problem Statement

The long term goal of messaging on the IC is to ensure that _canisters can rely on canister-to-canister messaging, irrespective of subnet load and with reasonable operational costs_.

This goal is impossible to achieve with the current messaging model; to the extent that [there were already discussions](https://forum.dfinity.org/t/fixing-incorrect-message-memory-fee/21987) about increasing prices for messaging on the IC. These discussions were paused to take a step back and see whether there are any other variables that could be tweaked to achieve the goal.

The reasons for why messaging on the IC doesn’t satisfy the long term goal are the following:

- Guaranteed response delivery implies unbounded response times. Concrete examples where this is a problem include calling into untrusted canisters, safe canister upgrades, and responsive applications in general. It also makes (true) fire-and-forget type of messages impossible. Later – once the IC supports more diverse subnet topologies – calling into dishonest subnets will also become a problem because of these guarantees.
- Relatively large upper bound on the size of requests/replies (2MB while the mean message size observed on mainnet is 1kB). In combination with guaranteed replies, this requires reserving orders of magnitude more memory than necessary in most practical cases, increasing costs both to the canister and to the system.

## Proposed Solution

The shortcomings above can be addressed by extending the current messaging model in two directions: _small messages with guaranteed responses_, and _best-effort messages_. The extensions will require explicit opt-in from canister developers so that backwards compatibility is maintained. Canisters that do not take any action will simply keep sending messages with the current semantics and guarantees.

### Small Messages with Guaranteed Responses

Small messages with guaranteed replies have the same semantics as existing canister-to-canister messages, except for being limited to 1 kB payloads. Besides being significantly less resource-intensive, the size restriction opens the possibility of ensuring every canister a quota of messages and thus a much more predictable environment. The current thinking is that it should be possible to give every canister guaranteed quotas of 50kB for incoming and 50kB for outgoing small messages that can be in flight at the same time, plus use of an optimistically shared pool of 5GB. (We assume an upper bound of 100k canisters per subnet. More would only be reasonable if they are part of one or more canister groups where a quota is no longer so important. Note that the guarantee to be able to produce an outgoing request does not change anything to the fact that delivery of requests is best-effort.) Initially small messages’ payloads will be limited to 1kB (50 incoming, 50 outgoing, 5M shared optimistically), but given demand this can be made more flexible later.

Small guaranteed-response messages still have the issue of potentially unbounded response times, but this may be an acceptable tradeoff in certain situations.

### Best-Effort Messages

For best-effort messages, both request and response delivery would be best-effort, which opens up the possibility for the system to ensure fairness even in high load scenarios via fair load shedding. Because the system may drop requests or responses under heavy load, memory reservations for responses are unnecessary. From a canister’s perspective every request still gets exactly one response. But the system does not guarantee that it is a globally unique response (e.g. the callee may produce a reply while the caller sees a timeout reject).

This means that canister developers who choose to rely on best-effort messages may have to handle the case where they can not infer from the reply whether the callee changed its state or not. In other words, best-effort messages allow developers to make a choice between bounded response times and the (potential) requirement to handle this additional case.

Additionally, every call has a deadline, set explicitly by the caller; or implicitly by the system. And when a response does not materialize before the deadline (whether because the callee did not produce one; or because the response got dropped) the subnet always generates and delivers a timeout reject response.

Similarly to small guaranteed-response messages, canisters would be guaranteed a quota of 50 concurrent best-effort calls, complemented by an optimistically shared pool of 5M calls.

## What’s in It for Me?

A predictable environment in terms of messaging, responsiveness, scalability, fairness, upgradeable canisters, safe interactions with untrusted canisters and malicious subnets. Eventually, sensible retries.

For more details see the accompanying [forum post](https://forum.dfinity.org/t/scalable-messaging-model/26920).

## Conclusion & Next Steps

The new extensions to the messaging model will provide an environment canisters are able to rely on, and, hence, make it easier to implement reliable and/or consistent cross-canister applications.

The scope of this proposal is the high-level direction to evolve the protocol outlined above. Assuming the proposal is accepted, we will keep working on the interface details and follow up as soon as they are worked out (link to the proposed changes to be posted on the forum). With that in place, the goal is to start working on a first iteration towards an MVP in the replica; and expose it in Motoko and the Rust CDK. Discussions on technical details will continue in the [forum discussion](https://forum.dfinity.org/t/scalable-messaging-model/26920).

Work beyond an MVP will be prioritized based on real-world community and system needs: besides completing the vision outlined in this post, we believe that there will be demand for fair load shedding; a `sleep` API; rejecting pending calls on stop; dropping the payloads of expired messages from blocks; bidding for message priority; etc.