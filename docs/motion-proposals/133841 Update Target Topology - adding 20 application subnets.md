## Motion Proposal - Adjustment of IC Target Topology to add 20 application subnets

The IC Target Topology sets targets for the number of Gen1 and Gen2 nodes per subnet and the decentralization coefficients (Nakamato coefficients) per subnet. It is a model used in the Internet Computer network to optimize the balance between node rewards and decentralization, and is not fixed and is voted in by the community as the target to be achieved within a certain timeframe. The latest approved IC Target Topology can be found in [this forum post](https://forum.dfinity.org/t/adjustment-of-ic-target-topology-to-increase-subnet-size-of-fiduciary-and-ii-subnets/34210) and was approved in [this proposal](https://dashboard.internetcomputer.org/proposal/132136).

## Background

Given the rapidly increasing load on the IC, we would like to propose adding additional application subnets to provide developers with greater flexibility and choice when selecting a deployment environment. This expansion will better position the IC network to accommodate continued growth in the coming months.

The following three step approach is taken to further expand the availability of accessibility of subnets for developers:

- Step 1: Expand the list of public subnets. This step has already been completed
- Step 2: Open subsets of the currently verified subnets to the community. Of the existing subnets, several so-called verified subnets are not open for deployment because these contain existing legacy canisters, some of which depend on a special replica configuration. These verified subnets - of which there are 11 - will be gradually opened.
- Step 3: Extend the target topology by 20 additional application subnets and gradually submit proposals to create these subnets as capacity is needed.

Further discussion on this approach can be found in [this forum post](https://forum.dfinity.org/t/suggested-approach-to-make-more-compute-capacity-available-on-the-ic/36567).

This motion proposal covers step 3 of the above approach, which is updating the target topology by adding application subnets to the IC Target Topology. It is proposed to add an additional 20 application subnets of each 13 node machines, based on the currently available node machine on the IC network.

## Proposed Target Topology

The below table shows the proposed update of the target topology.

|**Subnet Type**|**# Subnets**|**# Nodes in subnet**|**Total**|**SEV**|**Subnet limit NP, DC, DC Provider***|**Subnet limit country**|
|---|---|---|---|---|---|---|
|NNS|1|43|43|no|1|3|
|SNS|1|34|34|no|1|3|
|Fiduciary|1|34|34|no|1|3|
|Internet Identity|1|34|34|yes|1|3|
|ECDSA signing|1|28|28|yes|1|3|
|ECDSA backup|1|28|28|yes|1|3|
|Bitcoin canister|1|13|13|no|1|2|
|European subnet|1|13|13|yes|1|2|
|Swiss subnet|1|13|13|yes|1|13|
|Application subnet|51|13|663|no|1|2|
|Reserve nodes Gen1|||100||||
|Reserve nodes Gen2|||20||||
|**Total**|||**1023**||||

The latest approved IC Target Topology can be found in [this forum post](https://forum.dfinity.org/t/adjustment-of-ic-target-topology-to-increase-subnet-size-of-fiduciary-and-ii-subnets/34210) and was approved in [this proposal](https://dashboard.internetcomputer.org/proposal/132136).
