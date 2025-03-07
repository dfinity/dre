### **TL;DR** Reduce the minimum dissolve delay for staking on the NNS to 3 months

Following @dominicwilliams post on [possible optimizations of the NNS tokenomics](https://forum.dfinity.org/t/possible-optimizations-of-nns-tokenomics-updated/30352) and @bjoernek analysis on [possible optimizations of tokenomics](https://forum.dfinity.org/t/analysis-of-proposals-on-neuron-dissolve-delays-and-exchange-maturity/30890), it seems clear that reducing the minimum dissolve delay from 6 months to 3 months would benefit the IC. The feedback from the forum was positive, hence this proposal submission.

@bjoernek conclusion on this proposal was the following:

> **Proposal 1: Reduced Dissolve Delays** The analysis in this post shows that he primary method to balance supply and demand within the ICP ecosystem remains the growth of the cycle burn rate. Adjustments to inflation are likely to have a lesser impact. Reacting to community concerns, Proposal 1 was adjusted with the inclusion of an opt-in mechanism. Assuming that 25% of neurons opt in, the proposed changes would result in a modest 3.8% reduction in voting rewards. Given this limited impact, pursuing this proposal further may not be worthwhile. It might be sensible to revisit the voting reward function at a later point in time. However, a specific element of the proposal—reducing the minimum dissolve delay to three months—could be considered separately. This adjustment aims to attract new stakers by lowering the barrier to entry and merits discussion in a dedicated forum thread.

Another effect of that change is making it easier to go in and out of staking, which could benefit liquid staking protocols like @[WaterNeuron](https://x.com/WaterNeuron).

Following a quick analysis, the **APY for locking ICP for 3 months would be 7.66%.**

## Suggested Implementation

It seems that this change is pretty straightforward and would only require one change to the constant [here](https://sourcegraph.com/github.com/dfinity/ic/-/blob/rs/nns/governance/src/governance.rs?L157).

All the frontends and other places where this constant is used would need to update this constant as well.