## What is instrumentation?

The IC instruments Wasm canister binaries by injecting tiny snippets of code in order to count the number of executed instructions. This is needed to ensure that canister execution terminates and is fairly charged for.

More information about this and benchmarking results can be found in the [associated forum post](https://forum.dfinity.org/t/new-wasm-instrumentation/22080).

## Problem statement

The current instrumentation algorithm works well, but is inefficient for certain kinds of applications such as language interpreters. Also, the former instrumentation treats all instructions as equal, which may result in unfairness. Moreover, the current instrumentation is less than optimal when it comes to bounding execution round duration. This may result in lower block rate and latency increase for the end users. The new Wasm instrumentation addresses these inefficiencies and achieves an order of magnitude better performance for language interpreters, allowing developers to run their canisters more efficiently and possibly cheaper, as well as improved fairness and block rate.

## Proposed solution and changes

We propose a redesign of the way the IC performs instrumentation, modifying its core algorithm as well as important algorithm parameters, such as weights to be taken into account for Wasm instructions and system calls. The end result will be more efficient execution of user code, more stable block rate, and fair resource sharing between IC users.

Whereas the old instrumentation treats all instructions as equal in terms of cost, the actual cost of an instruction depends on its type. For example, division is more expensive than addition. The standard practice in the blockchain world is to have different costs for different instructions, for example varying gas costs per opcode in the EVM, as specified in the [Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf). Therefore, in the reworking of the instrumentation component we propose to take instruction weights into account when doing instruction counting. This leads to a non-uniform instruction cost model that might affect the total cycle consumption of certain workloads.

The proposed new instrumentation is already implemented and has been tested thoroughly. There are more details about this in the linked forum post. The new instrumentation is not enabled on mainnet. This proposal aims to align the community on whether the new instrumentation could be enabled. Specifically, the proposed changes for the new instrumentation and its parameters are:

1. The new instrumentation algorithm – found in the proposed [code](https://github.com/dfinity/ic/commit/4cb8960512763da6fe995b77a0780944dbf26273).
2. Instruction weights are re-calibrated in several changes: [code](https://github.com/dfinity/ic/commit/82ab6b8ec3521d3df62ae2db206e4cc87f0aebc8), [code](https://github.com/dfinity/ic/commit/9d106b8064863f58a80567ac42c79c06efb942b2), [code](https://github.com/dfinity/ic/commit/857978f942e1ad392ae3117dc8c3d6fb1022d4f4), [code](https://github.com/dfinity/ic/commit/4e46b7ca8db9168656da47692b61f58488825ef1).

## What are we asking the community

- Follow the [instructions](https://forum.dfinity.org/t/new-wasm-instrumentation/22080) to try locally the new instrumentation and check if and how your application is impacted.
- Participate in technical [discussions](https://forum.dfinity.org/t/new-wasm-instrumentation/22080) as the motion moves forward.
- Vote accept or reject on NNS Motion.