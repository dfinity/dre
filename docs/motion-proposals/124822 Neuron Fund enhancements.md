## Proposal Overview

We propose several enhancements to the Neurons’ Fund based on collected experience and community feedback.

- **Matched Funding**: Shift from a fixed ICP amount to a model where the fund’s contribution to SNS swaps scales with direct participation.
- **10% Participation Cap**: Introduce a cap ensuring the fund’s contribution to an SNS does not exceed 10% of total available funds at the time of proposal execution. Consequently, this automatically adjusts the fund’s participation if neurons opt out.
- **Distinct Swap Contributions**: Clearly separate contributions from direct participants and the Neurons’ Fund in both the SNS swap proposal structure and the SNS launch pad in the NNS frontend dapp.
- **Reduced Swap Duration**: Decrease the maximum swap duration from 90 days to 14 days to prevent fund blockage by potentially unsuccessful swaps.

## How Does Matched Funding Work?

### Matching function, f

The Matched Funding model employs an S-shaped matching function f, where x signifies the direct participation, and f(x) represents the Neurons’ Fund (NF) contribution. The function is characterized by:

- **Bounding Condition**: Bounding Condition: To ensure the NF’s contribution never surpasses the direct participation, f(x) is bounded by the function g(x)=x. Additionally, it remains below a cap, defined as the minimum of the ICP equivalent of 1M USD or 10% of the NF’s maturity. This ensures no single SNS drains the NF excessively.
- **Initial Lag Phase (I)**: Initially, f(x) stays at 0 until direct participation reaches an ICP equivalent of 100k USD, denoted as threshold t1. It then steadily rises until it hits an ICP equivalent of 300k USD (threshold t2), at which point the SNS receives a 2:1 contribution from the NF. This phase encourages projects to attract more direct participation.
- **Growth Phase (II)**: The NF’s contribution rises faster, providing more support to viable projects. When direct participation reaches the ICP equivalent of 500k USD (threshold t3), the SNS receives a 1:1 NF contribution.
- **Saturation Phase (III)**: Beyond the threshold t3, the growth rate of f(x) diminishes. Once direct participation exceeds threshold t4, which is twice the cap, f(x) levels off at the cap.

For a visual representation of the suggested matching function (which cannot be included directly in the proposal), please refer to this [link](https://forum.dfinity.org/t/suggested-enhancements-to-the-community-fund/20411/16).

The above-mentioned thresholds should be configurable as NNS parameters. Initially, these thresholds might be denominated in units of ICP, but eventually they should be denominated in terms of XDR.

### Benefits of Matched Funding

- **Better Reflection of Market Signals**: The matched funding system is designed to closely align with market sentiment. Specifically, a project that successfully raises more direct contributions will correspondingly receive a greater contribution from the NF, up to a predetermined threshold.
- **Streamlined Decision-making for NF NNS Neurons**: The automated adjustment feature in the NF’s contributions lessens the decision-making burden on NF neurons. As a result, these neurons have fewer instances where they need to opt out, making the process more efficient.
- **Improved Incentives for Projects**: The matching system provides a more compelling incentive structure for projects. Knowing that increased direct funding will be matched (up to a point) by the NF, encourages projects to be more proactive in their fundraising efforts.

## Community Engagement

The suggested enhancements have been syndicated in the forum. Find more details in [this thread](https://forum.dfinity.org/t/suggested-enhancements-to-the-community-fund/20411).