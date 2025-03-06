## Problem statement

With the approval of the [IC target topology](https://dashboard.internetcomputer.org/proposal/125549), a target of 750 node machines in the IC network is defined for the next half year/year. This target is defined keeping in mind that several of the Gen-1 node provider agreements will expire in the next two years.

Although the IC target topology requires fewer node machines than the current number of node machines (750 node machines vs approximately 1300 node machines), the IC network still requires an extensive number of Gen-1 node machines for operating purposes.

The long term objective is to have remuneration based on useful work for all node machines, which means node rewards are paid out based on the actual contribution to the IC, e.g. the number of blocks created, the size of the blocks created, how many times the node machine has been a block maker, in which subnet the node machine is running, etc; regardless of the type of node machine and when the node machine was bought. Since implementing this new approach to remuneration requires extensive discussion within the community as well as time to design and develop, an interim approach is required for the remuneration of Gen-1 node machines for which the node provider agreements will expire.

Despite the introduction of Gen-2 node machines, the Gen-1 node machines are still very relevant for the IC network for several reasons:

- They provide for the necessary decentralization of the IC network.
- Not all subnets require SEV-SNP functionality (the additional security functionality introduced with Gen-2 node machines).
- Since the initial capital investments for the Gen-1 node machines have been amortized, Gen-1 node machines are economically very attractive to operate.
- They provide for a buffer to scale up the IC network should use of the network start to increase sharply.

On the other hand, Gen-1 node machines in the IC network have several constraints:

- They cannot be deployed in every IC subnet since some subnets will require node machines with SEV-SNP support.
- As described in the forum posts on node diversification (see [node diversification part 1](https://forum.dfinity.org/t/ic-topology-series-node-diversification-part-i/23402), and [node diversification part 2](https://forum.dfinity.org/t/ic-topology-node-diversification-part-ii/23553)) - Gen-1 node machines are less decentralized and more concentrated at fewer node providers than Gen 2 node machines.
- There are too many Gen-1 node machines to fit the IC target topology.

## Proposal

Taking into account both the benefits and constraints of Gen-2 node machines, the following interim remuneration scheme for Gen-1 node machines after 48 months is proposed:

- **Rewards are optimized for 28 node machines** - if all Gen-1 node provider agreements have reached 48 months, it can be calculated that with a maximum of 28 nodes per Gen-1 Node provider, sufficient node machines remain in the IC network to meet the target topology of 750 node machines. However, it will still be possible for a Node Provider to continue to operate up to 42 node machines (similar as for Gen-2 Node Providers, and described in node diversification part 2), for example in anticipation of growth of the IC network and increase in ICP token price.
- **Rewards for Gen-1 node machines are lower than at launch** - rewards for Gen-1 node machines are lower than the rewards set at launch because of several reasons: not all Gen-1 node machines add to decentralization, Gen-1 node machines cannot be deployed in every subnet, and the initial investment costs for buying the node machines have been amortized by the node provider.
- **Rewards apply for a period of 12 months** - the interim remuneration proposal applies for a period of 12 months, after which the scheme will be reevaluated based on feedback and input from the community.
- **Rewards for Gen-1 node machines follow a similar formula as the rewards scheme for Gen-2 node machines** - node rewards will follow the same formula as remuneration for Gen-2 node machines, which is Initial reward for first node machine x Multiplier x Reduction Coefficient.

The Gen-1 node machine rewards are set at the values specified in the below table. To summarize the remuneration scheme, for a geography g, let

**mult(g)** be the geography multiplier

**value(g)** be the base value for a Gen-1 node in XDR

**r(np, g)** be the reduction coefficient

Then the monthly reward for the n-th node of a Node Provider (np) in geography g are defined as follows:

**reward(g, n)** = value(g) * mult(g) * r(np, g) ^ (n-1)

With a multiplier of 2.5 on the base value of the node, and a reduction coefficient of 0.97, this optimum of 28 node machines as described above can be achieved. The following table shows the geography-dependent values and the monthly reward for the first node onboarded. A few previously-separated geographic areas have been combined:

|Geography|Gen-2 value per node before multiplier for comparison|Reduced value for non- SEV-SMP nodes|Multiplier|Monthly reward for 1st node|Reduction coefficient r|
|---|---|---|---|---|---|
|US - California|771|496|2.5|1247.5|0.97|
|US - other|647|465|2.5|1162.5|0.97|
|Canada|771|496|2.5|1247.5|0.97|
|Europe|771|496|2.5|1247.5|0.97|
|Japan and Singapore|844|568|2.5|1420|0.97|

The above formula and table can be used to calculate the accumulated profit for each additional node. When calculating the accumulated profit for Gen-1 node machines in the United States, the below graph results, which shows the total profit for all machines up to the n-th node machine. It shows that when 28 nodes (2 racks of node machines) are kept on the IC network, almost maximum profit is achieved (30 to 31 node machines being the optimal).