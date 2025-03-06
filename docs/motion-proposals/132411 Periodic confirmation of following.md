## TL;DR

- This proposal replaces proposal 55651 with an updated and more detailed design.
- A neuron has to regularly take one of the following actions: directly vote, set following, confirm following.
- If a neuron fails to do one of these actions for 6 months, the neurons’ voting power will be decreased until it reaches zero after 7 months of inactivity. At this point the neuron’s following settings are reset.

## Context

This proposal is the result of [this forum discussion](https://forum.dfinity.org/t/periodic-confirmation-design/34215).

This proposal refines the design from proposal [55651](https://dashboard.internetcomputer.org/proposal/55651) to make it actionable. Adopting this proposal means that the proposal 55651 and the decision to adopt it are replaced with this proposal’s content.

## Design

The main idea of periodic confirmation is that in order to get rewards, governance participants have to remain active voters and regularly confirm their following settings. Neurons who set following once and then never interact with the NNS again get lower, adjusted voting rewards. Neurons who were created with default following and never made an active decision who to follow, have to do so in order to keep getting voting rewards.

1. In order to have voting power and get voting rewards, a neuron has to **regularly vote directly, set following, or confirm its current following settings**.
    
2. If a neuron does not take any of the above actions, the **neuron’s voting power is adjusted**. After **6 months of no action**, the neuron’ voting power is linearly decreased for one month until it **reaches zero at the end of 7 months** without any action. After these 7 months, the neuron’s following settings are fully reset.
    

To track its activity, governance remembers for each neuron the **last time when it took any of the above actions**.

### Relevant actions

In addition to confirmation of following, the design includes set following and direct voting. Setting following also reflects an explicit choice by the neuron who to follow. Direct voting is included because a neuron that always votes directly and does not rely on following is a very active governance participant and should not have adjusted voting power or adjusted voting rewards.

### Voting power adjustment

“Sleeper” neurons, who don’t take any of the above actions for more than 7 months, should not be automatically participating in voting and getting voting rewards. This design suggests to realize this by adjusting their voting power and resetting their followees.

#### Voting power adjustment

This design proposes that

- For each proposal and neuron, in the **ballot** consider the **adjusted voting power**. That is, record less voting power for neurons that have not taken any of the above actions for the last 6 months.
- For each proposal, distinguish
    - the **total (potential) voting power**, which is the sum of all neurons’ voting power without the adjustment
    - the **total adjusted voting power**, which is the sum of all neurons’ adjusted voting power that can contribute to the decision
- For each proposal,
    - **consider the total adjusted voting power for deciding proposals**.
    - **consider the total (potential) voting power when computing the rewards**. This is similar to the current design in that the rewards take into account the voting power if all neurons participated.

#### Effect of the voting power adjustment

Adjusting the voting power in this way has the following consequences:

- Sleeper-neurons are not considered in the decision making process. This means that proposals can still be decided quickly if the majority of the regularly active voters agree quickly.
- If a neuron has been sleeping for more than 7 months, then the voting power recorded for the neuron in any open proposal is zero. This has the advantage that neurons who have been inactive for a long time cannot simply show up and swing a proposal in an unexpected way.

### Who can perform the relevant actions?

Already now, direct voting and setting of following can be done by a neuron’s controller or its hotkeys. The same permission should apply for confirmation of following.

This means that a neuron can take any of the relevant actions (which will also reset the timer) with its controller or hotkey, which is in line with the original proposal and ensures that the feature can be implemented in a user-friendly way.

## Alternatives considered

1. Reset of following without voting power adjustment.
    
    Compared to the current design, this has the disadvantage that sleepers would still be counted towards the total decision voting power. As a consequence, if there are many sleepers, it would be hard or impossible to reach quick decisions for urgent proposals such as fixes to security bugs.
    
2. A one-off reset, possibly also with batches of neurons being reset at a time.
    
    This has similar disadvantages wrt quick decisions as above. Moreover, it does not achieve the goal that this is a periodic task for participants.
    
3. Adjustment of the voting rewards, but not the voting power.
    
    This would have similar disadvantages as explained for the reset of following above. In addition, this would mean that sleeper neurons would still contribute to the decisions with their full voting power, which does not seem to be in line with the original proposal’s intention.
    
4. Alternative time periods.
    
    Alternatives to the 6 and 7 months were discussed. The period before the adjustment should be short enough to have some effect but long enough so that it is realistic for most neurons to perform one of the actions without being a major hurdle. In the discussion it was argued that a biannual confirmation is easy to remember and meets these arguments.
    

## Future reward changes

This design does not change how rewards work and keeps the status quo. The motion does not exclude changing rewards going forward. Any other changes in how rewards work should be discussed, agreed upon, and implemented separately.