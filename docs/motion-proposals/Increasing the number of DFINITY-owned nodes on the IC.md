## Motion Proposal Draft: Increase of DFINITY-Owned Nodes and NNS Exception in Target Topology

**Title:**
Motion Proposal – Increase DFINITY-Owned Nodes to 70 and Allow NNS Subnet Exception

**Summary:**
This motion proposal requests that DFINITY be permitted to expand its operating node count from 42 to 70. In addition, it seeks an exception for the NNS subnet, allowing it to include 3 DFINITY nodes that are exempt from the standard topology restrictions (i.e., the limits of one node per data center owner, per data center, and per node provider). These modifications are essential to ensure sufficient spare capacity for maintenance, support ongoing redeployments, and facilitate the planned expansion of application subnets.

**Background:**

- **Current Configuration:** DFINITY currently has 42 nodes on the IC Mainnet, which comprises of 37 subnets. The current recovery process expects one DFINITY node per subnet in general, and exceptionally three DFINITY nodes for the NNS subnet for enhanced recovery resilience. This arrangement results in 39 nodes actively deployed in subnets, leaving only 3 nodes as spares.
- **Operational Challenge:** Among the 42 nodes, one node in Stockholm is degraded, and several nodes in Zürich are being redeployed via the HSM-less process (a procedure that temporarily removes nodes from an active subnet). Consequently, the effective spare capacity is reduced to 2 healthy nodes.
- **Future Expansion:** As per motion proposal [133841](https://forum.dfinity.org/t/adjustment-of-ic-target-topology-to-add-20-application-subnets/), plans to add up to 20 application subnets necessitate additional DFINITY nodes for each additional application subnet.

**Proposal:**

1. **Increase Total Node Count:**
    Approve the expansion of DFINITY-owned nodes from 42 to 70 by adding an additional 28 nodes. To maintain fairness among node providers (who are limited to 42 nodes), **the additional 28 nodes will not be eligible for rewards**.
2. **NNS Topology Exception:**
    Approve a modification to the adopted Target Topology that formally permits DFINITY to operate 3 nodes in the NNS subnet outside the regular topology restrictions. This exception will provide the necessary flexibility to maintain recovery resilience and manage operational challenges without extending recovery times.

**Proposed Target Topology Table (Adjusted):**

| **Subnet Type**    | **# Subnets** | **# Nodes in Subnet** | **Total Nodes** | **SEV** | **Subnet Limit (NP, DC, DC Provider)** | **Subnet Limit (Country)** |
| ------------------ | ------------- | --------------------- | --------------- | ------- | -------------------------------------- | -------------------------- |
| **NNS**            | 1             | 43                    | 43              | no      | 1* (with exception for DFINITY nodes)  | 3                          |
| SNS                | 1             | 34                    | 34              | no      | 1                                      | 3                          |
| Fiduciary          | 1             | 34                    | 34              | no      | 1                                      | 3                          |
| Internet Identity  | 1             | 34                    | 34              | yes     | 1                                      | 3                          |
| ECDSA Signing      | 1             | 28                    | 28              | yes     | 1                                      | 3                          |
| ECDSA Backup       | 1             | 28                    | 28              | yes     | 1                                      | 3                          |
| Bitcoin Canister   | 1             | 13                    | 13              | no      | 1                                      | 2                          |
| European Subnet    | 1             | 13                    | 13              | yes     | 1                                      | 2                          |
| Swiss Subnet       | 1             | 13                    | 13              | yes     | 1                                      | 13                         |
| Application Subnet | 51            | 13                    | 663             | no      | 1                                      | 2                          |
| Reserve Nodes Gen1 | –             | –                     | 100             | –       | –                                      | –                          |
| Reserve Nodes Gen2 | –             | –                     | 20              | –       | –                                      | –                          |
| **Total**          |               |                       | **1023**        |         |                                        |                            |

*Note: The NNS subnet includes 3 nodes operated by DFINITY that are exempt from the standard “1 node per DC/NP” rule.

**Benefits and Risks:**

- **Benefits:**
    - _Operational Resilience:_ Expanding the node count provides additional spare capacity, ensuring that maintenance and redeployment can occur without significantly delaying subnet recovery.
    - _Future-Proofing:_ The increased capacity supports planned expansion, including the addition of up to 20 application subnets.
    - _Clarification of Topology Rules:_ Formalizing the NNS subnet exception will resolve ongoing community questions and create a clear reference point for subnet membership change NNS proposals.
- **Risks:**
    - _Deviation from Standard Topology:_ Allowing an exception in the NNS subnet introduces a moderate departure from the established topology rules. However, given the operational challenges, this deviation is justified by the need for improved resilience.

**Conclusion:**
For the reasons outlined above, we urge the community to support this motion proposal to increase the total number of DFINITY-owned nodes to 70 and to allow the NNS subnet an exceptional status with 3 nodes not subject to standard topology restrictions. These changes are crucial to maintain network resilience and accommodate future expansion.

## Community Discussion
Link to forum discussion: [Increasing DFINITY Node Count and NNS Topology Exception](https://forum.dfinity.org/t/increasing-dfinity-node-count-and-nns-topology-exception/41654/1)
Discussion period: 26/02/2025 - 07/03/2025
