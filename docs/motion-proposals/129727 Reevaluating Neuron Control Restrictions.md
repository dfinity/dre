### TL;DR

The Internet Computer Protocol prevents canisters from directly controlling neurons, a measure intended to block neuron sales and ensure long-term thinking in voting behavior. However, these restrictions can be circumvented by a canister using threshold ECDSA (tECDSA) and HTTP outcalls. This suggests a need to reconsider the restrictions and their effectiveness.

This proposal recommends lifting restrictions on canister neuron control in the NNS and monitoring their materiality through newly developed metrics. A threshold is set to initiate mitigation measures if canister-controlled neurons exceed 10% of total voting power. Additional measures to disincentive neuron transfers will be considered and implemented if this threshold is reached, balancing security enhancements with user complexity and implementation effort.

### Background

**Restrictions on Neuron Control**

In the ICP network, neurons are decision-making entities created through the staking of ICP tokens. These neurons participate in governance by voting on proposals. To promote long-term decision-making, users are incentivized to stake their tokens for several years. Additionally, control over neurons is deliberately restricted: Currently only so-called [self-authenticating principals](https://wiki.internetcomputer.org/wiki/Principal#Self-Authenticating_principal) can be set as the controller of a neuron. A self-authenticating principal is an entity that utilizes its own cryptographic key pair (consisting of a private and a public key) to authenticate itself; the idea being that control over the underlying private key cannot be transferred without full trust. For example, a user relying on Internet Identity, Quill or a Ledger hardware wallet is of this kind. Canisters, which do not possess self-authenticating principals, are therefore excluded from directly controlling neurons.

**Reason for the Restrictions on Neuron Control**

The restriction is based on the requirement that neurons should not be sold - when canisters can control neurons directly, one can sell a neuron by selling the canister that controls it. It is considered important for neurons to be non-transferable/no-sellable because

- neurons should have an incentive to vote in the long-term interest of the Internet Computer and
- to avoid the possibility of attacks where an attacker acquires tokens only for a short amount of time, votes on a malicious proposal (e.g. transfer tokens) and then sells the neurons again.

**Circumventing Restrictions on Neuron Control**

Despite these safeguards, there are a few ways to bypass the restrictions on neuron control:

- Threshold ECDSA ([tECDSA](https://internetcomputer.org/docs/current/references/t-ecdsa-how-it-works/)): As canisters are able to control tECDSA keys, which are a feature of the Internet Computer Protocol, they can also sign ingress messages to the Internet Computer and thereby act as the controller of a neuron (making calls via HTTP to appear as ingress).
- Canister signatures: A canister can control a neuron through canister signatures, again making calls via HTTP to appear as ingress.

Furthermore, it is also possible to control a neuron via an Internet Identity (II) and [sell the II](https://xdtth-dyaaa-aaaah-qc73q-cai.raw.icp0.io/).

### Revisiting Neuron Control Restrictions

Given that the restriction for canisters to not control neurons can be circumvented relatively easily, it should be considered to drop that restriction. This was suggested by several ICP community members already. Lifting that restriction would bring the following benefits

- Facilitate NNS neurons that are SNS controlled: SNSs already chose to do this (e.g. OpenChat, GoldDAO), providing them a continuous income to cover cycles fees and involving SNSs in NNS governance. Allowing direct canister control of neurons would simplify the current more complicated workflow via tECDSA.
- Consistency with SNS: As opposed to the NNS, the SNS framework does not apply restrictions on neuron controllers.
- Facilitate organizational neuron ownership: An organization could control a neuron via a canister.

A key point discussed in the [forum](https://forum.dfinity.org/t/reevaluating-neuron-control-restrictions/28597) is the issue of materiality: As long as only a small number of neurons are controlled by canisters or through II, this does not significantly impact long-term voting behavior of the NNS overall.

### Suggested Way Forward

Acknowledging the potential for circumvention of the current mechanism, it is recommended to lift the restrictions on canister neuron control while monitoring the materiality of canister-controlled neurons, via the following steps:

- Lift restrictions on neuron control: Remove existing restrictions, allowing canisters to control NNS neurons.
- Implement new metrics: Develop and implement metrics in the NNS governance canister that track the total stake and voting power of canister-controlled neurons on a daily basis. This should include the proportion of canister-controlled voting power relative to the total voting power.
- Establish a materiality threshold: Set a threshold that triggers mitigation measures if canister-controlled neurons exceed 10% of the total voting power. The threshold might be adjusted at a later point in time. For example, the threshold might be increased (upon NNS approval) following a materiality analysis of canister-controlled neurons belonging to DAOs, which are not considered to be an issue.
- Delayed implementation of additional measures: Additional disincentives for neuron transfers will be implemented if the materiality threshold is surpassed. While further measures could enhance security, they would also increase complexity for users and require significant implementation effort. The specifics of these mitigation measures can be determined as the situation evolves and will be subject to a separate motion proposal. For instance, these disincentives would reduce the rewards for transferable neurons created after this proposal is passed; to avoid the reduction, non-transferability of neurons could be shown via a proof of knowledge of a cryptographic key controlling the neuron.