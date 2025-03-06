## TL;DR

Introduce public and private neurons and a neuron index

- Public neurons provide more transparency to followers
- Private neurons keep ballots confidential
- More transparency by revealing all neurons’ IDs and their stake

## Current situation

**For a neuron whose neuron ID is known, there is some information that everyone can get about the neuron, including:**

- The neuron’s stake and voting power
- The neuron’s voting power bonus in the form of the dissolve delay and age (and the neuron’s creation time)
- The neuron’s ballots
- Whether the neuron is a member of the Neurons’ Fund

**Not all neuron IDs of neurons in the NNS governance are known today.**

The reason for this is that whenever a new neuron is created, it gets a new random neuron ID. This neuron ID is only known to the user that created the neuron and to the NNS.

**This has the following consequences.**

1. If a **user would like to hide information about their neuron, they can keep their neuron ID secret.** This is an important feature to some users who would like to keep their ballots private, in order to vote freely and not fear consequences. For these neurons whose IDs are kept secret, no information is publicly known.
2. It is **not possible today to build an index of all neurons**. The community has adopted a proposal to build a neuron index in NNS governance ([https://dashboard.internetcomputer.org/proposal/48491](https://dashboard.internetcomputer.org/proposal/48491)). Once implemented, this allows users to get a comprehensive overview of how much each neuron stakes, allowing for example to get more insights with respect to how much stake is locked for how long.
3. If a user **would like to show details about their neuron to the public, there is no easy way to do so**. This means for example, that followers of a known neuron cannot easily query the NNS to learn settings of the known neuron, such as who the known neuron follows on different topics.

## Proposed new design

This proposal suggests a new design that addresses all needs above. For all neurons, it provides more transparency with respect to the stake. At the same time it maintains the users’ choice how much they would like to reveal with respect to their ballots where this is requested. Finally, for users who would like to reveal more information about their neuron, the new design provides an easy way to do so.

### Details of the new design

- For public neurons, everyone can read all fields of the neuron, except for the ICP ledger account ID. The reason for this exception is that this is particularly sensitive and can be used for in-depth ledger tracing.
- For private neurons, everyone can read the following fields
    - Neuron ID
    - The neuron’s stake and voting power
    - The neuron’s voting power bonus in the form of the dissolve delay and age (and the neuron’s creation time)
    - The neuron’s type indicating that the neuron is a seed or ECT neuron
- A neuron index in NNS governance lists all private and public neurons.
- Each neuron can choose to be private or public and change this at any point in time. The known neurons are public while all other neurons are set to private by default.

This design has the following consequences regarding the above mentioned user needs.

1. If a user would like to keep their voting behavior confidential, they can set their neuron to be private.
2. The neuron index provides more transparency by listing all neurons and important information about them, such as how much they stake.
3. If a user would like to show details about their neurons to the public, they can set their neuron to be public. This allows any neuron, but especially known neurons, to provide additional transparency about their behavior to their followers.

### Resulting properties

- This design increases the transparency with respect to the neurons’ stake and voting power.
- This design maintains the main verifiability properties of the voting process.
    - As is the case now, each user can verify that their vote was submitted (individual verifiability) and all users and outsiders can verify that the tallying process is correct by verifying the code (universal verifiability).
    - With respect to ballot visibility, the new design can make some ballots visible which are not visible now (unknown neuron IDs that chose to be public) and vice versa (neurons who choose to be private).