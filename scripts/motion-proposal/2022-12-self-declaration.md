# NP Self Declaration Motion proposal

## The problem

A secure and sustainable operation of the IC requires (1) maximal decentralization and (2) that Node Providers (NPs), who own and operate the network’s nodes, behave in the interest of the network.

Maximal decentralization means maximizing the number of the independent NPs and diversifying the geographies, jurisdictions, and datacenters (DCs) where the nodes are operated.

In a fully decentralized network, onboarding of a Node Provider (NP) is managed entirely by the Network Nervous Systems (NNS). This means that anybody who wants to become a NP needs to submit a proposal that will be voted upon by the community. The main question then is how to decide whether to accept or reject a new NP into the network.

## The goal: Assessment of a new NPs by the community

We propose to take a three level approach that will help the community to assess whether a new NP should be onboarded:

* Automatically validate part of the technical configuration of the NP during onboarding.
* Financial stake by the NP through investment in node hardware.
* Self-declaration of identity and good intent.

First, there are a lot of items that can be validated automatically during the onboarding of a NP, in particular the configuration setup. These can be included as part of the autonomous onboarding process that is currently being worked on.

Second, new NP will have a substantial stake in the IC through the investment in HW Infrastructure for running their nodes. NPs are not required to stake ICP, but the HW investment ensures that the NP has sufficient incentive to ensure to run nodes efficiently and reliably. This level of NP assessment has already been implemented through the current NP reward system, and will be refined in the future.

A financial stake as described above might not be enough to dissuade malicious node providers from colluding to break a subnet. Therefore, as a third level of assessment, we propose to the community that NPs are asked to present a self-declaration when requesting the NNS to be added to the network. In this, NPs:

* state their identity and (if applicable) business entity;
* accept that they are liable for the financial damage and harm caused in case they maliciously collude with other node providers to subvert the functioning of the network;
* accept that they understand that deliberately subverting the protocol, by modifying code, colluding with other malicious node providers, or otherwise, constitutes the misuse of a computer system.

We believe that a self-declaration will greatly increase security to the benefits of all stakeholders of the IC network, ranging from NPs to developers and entrepreneurs building on the network.

## Proposal

By adopting this governance proposal, the IC community agrees on a node provider self-declaration process. Future node provider candidates are asked to follow this process to reveal their identity and acknowledge their responsibility for the network.


## Documents and Templates

Before submitting a proposal to become a new NP, the NP shall prepare two kinds of documents. The documents should be delivered as a pdf document, digitally signed.

### (1) Self-declaration

#### (A) Statement of identity

Name: _________________________________________________________

Entity name: (in case of business entity) ____________________________

Official address and location: ______________________________________

Country: ________________________________________________________

Business registration number (in case of business entity): ________________

#### (B) Statement of provision of node machines

I hereby guarantee that I shall provide node machines in accordance with the required Hardware Configuration for running the IC Network, as described on the IC wiki (see https://wiki.internetcomputer.org/wiki/Node_provider_hardware).

#### (C) Statement of good intent

I guarantee to the world that I shall honestly operate the node machines I provide, and that should I behave dishonestly, for example by deliberately interfering with my node machine(s) to prevent them correctly processing ICP protocol messages, in collusion with others or alone, that I will be liable to users of the network, and to other node providers, for any damages caused.

I further declare I am aware that any deliberate interference with a node machine, which causes it to incorrectly process ICP protocol messages, represents a misuse of that hardware, and of any hardware it interacts with, and that in some jurisdictions, that may constitute a crime.

Signature of representatives: ___________________     ___________________


### (2) Identity Proof

The NP shall provide proof that the identity(ies) listed in the self-declaration exist in the real world. The proof can be the official business registration (in case of a business entity) or any document or internet-reference that sufficiently proves the identity of the signers of the self-declaration to the community.


## Process

Initially, the process is quite manual. Over time, it shall be automated and for convenience be incorporated into dApps running on the IC. For now:

1. Preparation: the NP prepares
* the two kinds of documents listed above in a format that is widely available, e.g. PDF
* creates a compressed file, e.g. zip file, including these documents and computes a hash of this file

2. Publication: the NP uploads the documents to the wikipage [[NP Self Declarations]].

3. Proposal submission: the NP submits a proposal to the NNS asking to be accepted to the network.
* The technical instructions are provided in the https://wiki.internetcomputer.org/wiki/Node_Provider_Onboarding#VII._Register_your_NP_principal_to_the_network
* The summary of the proposal shall point to the published file (step 2) and list the hash (step 1)

4. NNS vote: It’s now up to the NNS community to check whether the provided information matches the community’s expectations and to vote on the proposal

___

These templates and process description can also be found on the IC wiki: https://wiki.internetcomputer.org/wiki/Node_Provider_Self-declaration.

Developer forum: Prior to submission, this proposal was discussed with the community in the forum thread https://forum.dfinity.org/t/proposal-for-node-provider-self-declaration/16501

