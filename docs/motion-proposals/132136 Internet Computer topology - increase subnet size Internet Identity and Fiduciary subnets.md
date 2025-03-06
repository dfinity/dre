## Motion Proposal Adjustment of IC Target Topology to Increase Subnet Size of Fiduciary and II subnets

The IC Target Topology sets targets for the number of Gen1 and Gen2 nodes per subnet and the decentralization coefficients (Nakamato coefficients) per subnet. It is a model used in the Internet Computer network to optimize the balance between node rewards and decentralization, and is not fixed and is voted in by the community as the target to be achieved within a certain timeframe. The latest IC Target Topology can be found here 5.

It is proposed to update the IC target topology in order to increase the subnet size of the Fiduciary subnet and II subnet. The main reason for this is to increase the security of the IC since larger subnets require more node machines or node providers colluding with each other in order to take over these subnets. Given that the IC network currently has sufficient spare node machines, no additional node providers or node machines need to be onboarded. The intention is to gradually increase the size of these Fiduciary subnet and II subnet over time, and at a certain (to be determined) time set up specific production and backup subnets for storing the tECDSA signatures.

In addition to increasing the subnet size of the II and Fiduciary subnets, it is proposed to update the country limit for the Swiss subnet to 13, which was erroneously set to 2 in the currently approved IC Target Topology table.

Further discussion on this proposal can be found in [this forum post](https://forum.dfinity.org/t/adjustment-of-ic-target-topology-to-increase-subnet-size-of-fiduciary-and-ii-subnets/34210).

## Background

The uzr34 and pzp6e subnets are subnets with 28 active node machines. These subnets have more active node machines than an application subnet (that has 13 active node machines) since the uzr34 and pzp6e support important functionality for the IC:

- The uzr34 subnet runs the Internet Identity, Cycles Ledger, Exchange Rate canister, and XRC dashboard, and stores the backup tECDSA signing keys. The pzp6e subnet acts as a Fiduciary subnet and stores the tECDSA signing keys.
- With 28 nodes, | 28 / 3 | + 1 or 10 nodes need to be malicious in order to take control of these subnets. By adding more node machines to these subnets, more node machines/node providers need to collude to take over the subnet. Hence, adding more node machines to these subnets will add to the security of these subnets.

It is proposed to improve the security of these subnets by adding more node machines. This will mean an update of the IC target topology that was previously approved 1 on 12th November 23. As there are currently more node machines in the IC network than required to meet the decentralization targets set in the target topology, no new node machines will need to be onboarded, and spare node machines can be used to increase the size of the uzr34 and pzp6e subnets.

## Roll-out plan

With the currently available nodes, the uzr34 and pzp64 subnets can be increased to 34 node machines and still meet decentralization targets.

For the long term, the following roll-out plan is intended:

- Further increase the subnet size of uzr34 and pzp64 with 3 nodes every time when spare nodes are available. The reason for adding nodes in multiples of 3 is that each three nodes will result in one more additional node machine having to act maliciously to take over the subnet.
- As a final step in the roll-out plan, separate ECDSA signing and ECDSA backup subnets will be created, provided the load on the subnets requires to have separate subnets for ECDSA, and sufficient spare node machines are available to meet the decentralization coefficients.

## Proposed Target Topology

The below table shows the proposed update of the target topology. The original target topology can be found in [this forum post](https://forum.dfinity.org/t/ic-topology-node-diversification-part-ii/23553).

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
|Application subnet|31|13|403|no|1|2|
|Reserve nodes Gen1|||100||||
|Reserve nodes Gen2|||20||||
|**Total**|||**763**||||

In addition to increasing the subnet size of the II and Fiduciary subnets, it is proposed to update the country limit for the Swiss Subnet to 13, which was erroneously set to 2 in the currently approved IC Target Topology table. Note that there currently is no Swiss subnet yet implemented.