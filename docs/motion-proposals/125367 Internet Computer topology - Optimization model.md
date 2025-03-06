## Problem statement

The Internet Computer (IC) operates on physical node machines and achieves decentralization by partnering with multiple independent node providers. This decentralization reduces risks of central control, failures, and censorship. However, there is a trade-off: a high degree of decentralization comes at a cost in the form of node provider rewards to account for hardware, maintenance, and operations.

In order to strike the right balance between network size & cost and the degree of decentralization, the IC needs to tackle two aspects

- Establishing a Target Topology: This involves defining the number of subnets and their respective sizes, aligning with anticipated future demand. It also involves setting decentralization targets. A proposal for this target topology will be submitted in a separate motion proposal.
- Optimization: Given a target topology, use a model to optimize between node rewards (onboarding of additional new nodes and rewards for existing nodes) and decentralization.This is the primary focus of this motion proposal.

## Modeling approach

To address this problem, it is suggested to use the previously syndicated linear [model](https://forum.dfinity.org/t/ic-topology-node-diversification-part-ii/23553) for optimizing between node rewards (including onboarding new nodes and compensating existing ones) and network decentralization.

Inputs to the model include:

- Available nodes and their characteristics (node provider, data center, data center provider, country).
- The target topology that describes the desired network structure, detailing the count and size of subnets, and decentralization goals.

The model then calculates either the least number of new nodes or the minimal node rewards required to achieve these goals. A prototype implementation to run the model and create visuals is available.

## Model application process

It is suggested to use the model framework for making informed decisions regarding future node onboarding as follows:

**Establishing a Target Topology**

The NNS agrees on a single target topology at any given time. Various decisions, such as whether to onboard additional nodes, can be derived from this agreed-upon topology. As the IC evolves, updated target topologies could be proposed to the NNS, ensuring continual alignment with the network’s development and needs.

**Optimizing Node Allocation**

Utilizing the defined target topology, the model can determine the minimal number of nodes, or alternatively, the minimal amount of rewards required for achieving the target topology.

**Deciding on Node Candidates**

Utilizing the model, the following can be analyzed given a set of current nodes and node candidates:

- Effectiveness of Candidate Nodes: Determine if node candidates can directly reduce 1:1 the number of additional nodes needed.
- Node Relevance of Existing Nodes: Identify existing nodes that are not utilized to achieve the decentralization target, signaling potential candidates for offboarding.