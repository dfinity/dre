# Motion Proposal Target IC Topology

As part of the forum posts on IC topology (see [Forum post 1](https://forum.dfinity.org/t/ic-topology-series-node-diversification-part-i/23402/5) and [Forum post 2](https://forum.dfinity.org/t/ic-topology-node-diversification-part-ii/23553)) an optimization model designed to reach decentralization targets with a minimum node count as well as a IC target topology have been discussed within the IC community. This motion proposal is for the community to vote on the initial IC target topology. A separate [motion proposal](https://dashboard.internetcomputer.org/proposal/125367) has been submitted and accepted for the community to vote on the optimization model.

## Establishing a Target Topology

Should this proposal be adopted, the target topology as presented below and motivated in [Forum post 2](https://forum.dfinity.org/t/ic-topology-node-diversification-part-ii/23553) shall be used by the IC community to decide on future node onboarding in the following way:

- Target Subnet structure: This involves determining the number and respective sizes of subnets, aligned with anticipated future demand.
- Decentralization Targets: Per subnet, decentralization targets should be set, utilizing either a Nakamoto coefficient or a subnet limit. The node topology matrix, described in the previous Forum post 2, assists in evaluating achievable targets.

It is proposed that the NNS agrees on a single target topology at any given time. Various decisions, such as whether to onboard additional nodes, can then be derived from this agreed-upon topology, and potential node providers can get clarity. As the IC evolves, new target topologies can be proposed to the NNS, ensuring continual alignment with the network’s development and needs.

## Proposed Target Topology

The following table specifies the type, number, and size of anticipated subnets. The column labeled “SEV” indicates whether the subnet is designated to run on generation 2 SEV machines, enhancing protection against malicious actors. This table serves as a suggestion for the Subnet Target Structure for the next 6-12 months or until a new target topology is adopted by the NNS. The number of subnets is based on current and anticipated demand. There are some special subnets that are dedicated to specific use cases, e.g., ECDSA signing, SNS, and NNS. The sizes of the subnets were chosen depending on the sensitivity of the services/dapps running on them, e.g,. the NNS subnet has the highest sensitivity and is thus proposed to be larger than app subnets.

The following decentralization targets are proposed:

- The node provider, data center, and data center provider characteristics, will adhere to maximum decentralization (subnet limit = 1 where subnet limit is defined as the maximum number nodes with identical node provider, data center or data center providers in one specific subnet).
- The decentralization target for the country characteristic will be set to 3 for subnets with 28 or more nodes, and 2 for subnets consisting of 13 nodes. This ensures that both larger and smaller subnets have similar decentralization coefficients (Nakamoto coefficients) that are high enough for each type of subnet to have a high level of protection against malicious or colluding nodes. The country limit would permit the same country to appear up to twice in any 13 node subnet, and three times in a subnet of 28 or more nodes (country subnet limit = 2 or 3 where country subnet limit is defined as the maximum number of nodes from one specific country in one specific subnet).

|**Subnet Type**|**# Subnets**|**# Nodes in subnet**|**Total**|**SEV**|**Subnet limit NP, DC, DC Provider***|**Subnet limit country**|
|---|---|---|---|---|---|---|
|NNS|1|43|43|no|1|3|
|SNS|1|34|34|no|1|3|
|Fiduciary|1|28|28|no|1|3|
|Internet Identity|1|28|28|yes|1|3|
|ECDSA signing|1|28|28|yes|1|3|
|ECDSA backup|1|28|28|yes|1|3|
|Bitcoin canister|1|13|13|no|1|2|
|European subnet|1|13|13|yes|1|2|
|Swiss subnet|1|13|13|yes|1|2|
|Application subnet|31|13|403|no|1|2|
|Reserve nodes Gen1|||100||||
|Reserve nodes Gen2|||20||||
|**Total**|||**751**||||

*_Node Provider, Data Center, Data Center Provider_