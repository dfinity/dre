# SRC Motion Proposal

## Background

Multiple community members have expressed interest in renting an entire ICP subnet. There seem to be sufficient use cases that justify the introduction of subnet rental on the Internet Computer. Subnet rental means that a whole subnet can be rented by a tenant (a person or company). A rented subnet gives the tenant a) exclusive resource access and b) some choice over the geographic distribution of nodes, which in turn allows more regulatory certainty. The IC ecosystem benefits because subnets are rented for a fee that assumes full utilization, which will significantly increase the cycles burn rate, contributing to deflation.

The community, represented by the NNS, retains ownership of all IC resources, including subnets for rent. A rental agreement is therefore an agreement between a tenant and the NNS. This suggests the use of NNS proposals to create such an agreement. The rental process, in particular payments, should be handled by a smart contract, so that the solution is as autonomous as possible. This suggests a canister whose domain is subnet rental. This Subnet Rental Canister (SRC) is controlled by the NNS and installed on the NNS subnet, because it 1) handles a significant amount of cycles and 2) must be able to make privileged method calls on other NNS canisters, so the security standards of the SRC should at least match that of its dependencies (CMC, Registry canister, etc.).

## Details

Exclusive access to a subnet can be granted via whitelisting a user’s principal. Only this principal will be able to install canisters on the rented subnet. A method for this purpose already exists on the cycles minting canister. Disabling cycles consumption for the rented subnet can be achieved with a new Registry flag and small changes to the execution environment. The lifecycle of a rental agreement between the tenant and the NNS, as well as the payment flow, is the responsibility of the new subnet rental canister.

### Setup Process

In general, a subnet with a specific topology desired by the user may not yet exist. Therefore, the process described below includes the creation of a new subnet as part of the rental agreement setup.

Here is a timeline of the setup process:

1. The user who wishes to rent a subnet pays a deposit to an ICP ledger subaccount controlled by the SRC. The amount is defined in XDR, published on the SRC, and covers six months of node provider rewards, scaled by subnet size and including a surcharge. The XDR/ICP exchange rate used must be the exchange rate from the UTC-midnight before the proposal creation in step 2. In other words: No midnight should lie between the payment and the proposal creation.
2. The user creates an NNS proposal of type “Rental Subnet Request”, which contains their principal and either a topology description or an existing subnet id. The description may refer to the geographical features of the desired subnet and will be judged by the NNS community.
    1. If the proposal is rejected, the deposit is refunded
    2. If the proposal is accepted, the topology description is considered an addendum to the NNS-agreed target topology, which should facilitate the necessary node provider and node onboarding proposals. 10% of the deposit is considered non-refundable and is converted to cycles immediately. Every 30 days, another 10% of the initial amount is converted to cycles and becomes non-refundable. The remaining ICP are refundable up until the subnet is created. A refund can be initiated via a method “get_refund” on the SRC which may only be called by the principal specified in the NNS proposal.
    3. The proposal fails if the SRC does not find the deposit or it does not suffice (based on the XDR/ICP exchange rate from the UTC-midnight before the proposal creation).
3. Over the next weeks, new node providers may be onboarded, new nodes created etc.
4. When all necessary nodes exist, the user may create a subnet creation proposal. This existing proposal type is extended with an optional field “parent_proposal_id”. This field must contain the id of the initial proposal from step 2, so the SRC can detect the newly created subnet id and connect it to the rental agreement and the user’s principal. The subnet is now ready to be rented, so the SRC whitelists the user’s principal via a call to the CMC, converts the remaining ICP to cycles and starts burning cycles.

### Active Rental Phase

While a rental agreement is active, the SRC keeps track of the point in time until which the subnet is paid for, referred to as “covered_until” in the following. If this point in time lies less than three months in the future, the SRC attempts to convert ICP to cycles to cover for the next month. If it succeeds, “covered_until” is moved 30 days into the future. Otherwise, the SRC tries again the next day. If “covered_until” lies in the past, the SRC suspends services on the rented subnet (see below).

The cycles are burned at regular intervals during the rental period. The total cycles burn rate of the IC will increase significantly with every active rental agreement.

### Suspension

In case payments cease and funds run out, the SRC can autonomously suspend services on a rented subnet. Suspension is reversible and should not brick canisters that are developed robustly. It should, however, make running canisters unusable for the duration of the suspension, and prevent malicious canisters from doing any harmful work (including calling into other subnets).

In case potential malicious canisters on the rented subnet interfere with other subnets, the suspension mechanism can be used by the community to mitigate the problem at any time via a regular NNS proposal.

When a tenant no longer has use of a rented subnet, they stop paying, the SRC suspends the whole subnet and the community can decide how to reuse either the whole subnet or the nodes it comprises.

## Proposal

By adopting this proposal, the NNS agrees to adding the above described subnet rental capability to the ICP. This feature is realized with a new canister under NNS control, the subnet rental canister (SRC). A new proposal type “Subnet Rental Request” shall be added, which is executed on the SRC. The SRC processes and supervises the payment flow and can autonomously suspend the services on a rented subnet if payments cease.

Forum discussions on this feature can be found [here](https://forum.dfinity.org/t/subnet-rental-swiss-subnet/25773) and [here](https://forum.dfinity.org/t/subnet-rental-canister/28334).

Please vote to accept or reject this motion proposal.