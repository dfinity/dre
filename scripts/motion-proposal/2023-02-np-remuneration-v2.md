# Node Provider V2 Remuneration Model

This proposal describes the updated remuneration that is applicable to new Node Providers that implement nodes based on the Generation 2 Hardware specification, also called the V2 Remuneration Model. The remuneration is described in detail on this wiki page: https://wiki.internetcomputer.org/wiki/Node_Provider_Remuneration, and has been shared with the community in this forum thread: https://forum.dfinity.org/t/the-state-and-direction-of-decentralization-nodes-on-the-internet-computer/9170/128.

The V2 remuneration model accounts for the fact that the second generation of node hardware is significantly more expensive than the first generation.

The following updates are proposed:
- Higher rewards for the first nodes of a new Node Provider, in order to attract more NPs and improve ownership decentralization.
- More refined rewards for nodes in new geographies, such as South America, Africa, Asia and Australia, to stimulate further geographical decentralization.

We therefore propose to introduce a node reward model parametrized by:
- Geography multiplier (mult): This multiplier will be lower, namely 2, for regions with many nodes (e.g. Europe and North America), and higher, namely 3, for regions where there are currently limited nodes present (such as Africa and South America)
- Reduction coefficient (r): The node reward of the n-th node of a Node Provider is multiplied by `r ^ (n-1)`. The reduction coefficient r is dependent on the geography of the Node Provider. Consequently, the first node of a Node Provider receives attractive rewards but it becomes increasingly less attractive to add additional nodes.

The rewards are furthermore dependent on estimated capital and operational expenses that vary based on geographies. A table with the concrete numbers follows below.

In summary, for a geography g, let
- mult(g) be the geography multiplier
- cost(g) the total costs over 4 years for acquiring and maintaining a gen 2 node in g in XDR
- r(g) be the reduction coefficient

The monthly reward for the n-th node of a Node Provider in geography g are defined as follows:

```
reward(g, n) = cost(g) * mult(g) * (r(g) ^ (n-1)) / (4 * 12)
```

The total costs over 4 years are multiplied by the geography multiplier, multiplied by the reduction coefficient, and divided by 4 years times 12 months. As a result, rewards for nodes in new geographies and for Node Providers with few nodes are higher. Thereby, a geographical and ownership decentralization is incentivized. The following table shows the geography-dependent values and the monthly reward for the first node onboarded.


All prices are in XDR unless explicitly stated otherwise.
| Geography | Total cost over 4 years | Multiplier | Monthly reward first node | Reduction coefficient r |
| ----- | ----- | ----- | ----- | ----- |
| USA | 31034 | 2 | 1294 | 0.7 |
| US - FL/GA/CAN | 37031 | 2 | 1542 | 0.7 |
| EU | 38996 | 2 | 1542 | 0.95 |
| Asia Singapore | 40508 | 2 | 1688 | 0.7 |
| Asia non Singapore | 40508 | 3 | 2532 | 0.98 |
| South Africa | 43986 | 3 | 2748 | 0.98 |

The remuneration will be further updated and refined for new geographies, such as South America, Africa, and Australia as well. This is work in progress, and updates will be posted as a new motion proposal to be voted upon by the community.
