# Background & Problem statement

Canisters pay for storage dynamically every few minutes following the "pay-as-you-go" model. Such a fine-grained payment is convenient for developers, but at the same time it doesn’t handle spikey usage patterns well.

Consider a scenario where someone allocates the entire subnet storage for a few hours and pays only for those hours. The end result is that during those hours, the operation of other canisters on the same subnet might be disrupted as they might fail to allocate new storage. The problem here is that the cost of such a spikey usage is low due to the "pay-as-you-go" model.

The goal of this proposal is to address this long-standing problem with a new resource reservation mechanism that is designed to discourage the spiky usage pattern by making it more expensive while at the same time keeping costs the same for long-term users.

# Proposal

Recently the subnet storage capacity has been increased from `450GiB` to `700GiB`. The newly added `250GiB` is subject to a new resource reservation mechanism that works as follows:

- As long as the subnet remains under the previous limit of `450GiB`, the storage payment remains the same as before, following the "pay-as-you-go" model. This is the case for all subnets as of this writing.
- When the subnet grows above `450GiB`, then the new reservation mechanism activates. Every time a canister allocates new storage bytes, the system sets aside some amount of cycles from the main balance of the canister. These reserved cycles will be used to cover future payments for the newly allocated bytes. The reserved cycles are not transferable and the amount of reserved cycles depends on how full the subnet is. For example, it may cover days, months, or even years of payments for the newly allocated bytes. It is important to note that the reservation mechanism applies only to the newly allocated bytes and does not apply to the storage already in use by the canister.

Summary of the changes:

- A new field named `reserved_cycles` is added to the canister state.
- A new field name `reserved_cycles_limit` is added to canister settings.
- Storage allocation operations such as `memory.grow`, `stable_grow`, `stable64_grow`, setting `memory_allocation` are adjusted to move cycles from the main balance to `reserved_cycles`.
- The periodic charging for storage is adjusted to first burn cycles from `reserved_cycles` and only when that reaches 0, to start using the main balance.
- The freezing threshold computation is also updated to take `reserved_cycles` into account. This means that even if the main balance is below the freezing threshold, the canister may still be functional if it has enough `reserved_cycles`.

Detailed explanation of the changes is available in the linked [forum thread](https://forum.dfinity.org/t/23447).

Properties of the new reservation mechanism:

- It makes spikey usage of storage where a canister frequently allocates and deallocates storage more expensive. Note that currently the only way to deallocate storage is to uninstall, reinstall, or delete the canister, so it is not a common usage pattern.
- It has minimal impact on costs for canisters with long-term storage usage.
- It is backwards compatible in the sense that it activates only in the newly added `250GiB`. Nothing changes for canisters if their subnet remains below the old limit of `450GiB`.
- It seamlessly integrates with the allocation operations without changing their interfaces.
- Controllers can opt out by setting `reserved_cycles_limit` to zero. Such opted-out canisters would not be able to allocate from the newly added `250GiB`, which means that these canisters will trap if they try to allocate storage when the subnet usage grows above `450GiB`.

# Voting

Normally, changes in the protocol first go through the community discussion, then the NNS proposal, and then roll out to the mainnet. In this case, because of the possibility that this vulnerability of the protocol could be abused, DFINITY followed [the approach for security fixes](https://dashboard.internetcomputer.org/proposal/48792), where the fix is deployed first and then discussed later.

The new reservation mechanism has been implemented in the replica and is active in the newly added `250GiB`.

This vote is about whether to keep the new reservation mechanism or remove it. If the proposal is accepted, then the new reservation mechanism will remain in replica and will be added to the interface specification of the Internet Computer. If the proposal is rejected, then the new reservation mechanism will be removed and an alternative fix for vulnerability would need to be found and implemented.