
# Description

Submit an NNS motion proposal from the provided markdown file

# Example usage

```
❯ ./submit-motion-proposal.sh 2023-05-np-remuneration-v3.md
!!! Dry run
!!! ****************************************************************
!!! * NOTE: invalid proposer is intentional, please ignore the error
!!! ****************************************************************

+ dfx --identity proposals canister --network ic call rrkah-fqaaa-aaaaa-aaaaq-cai manage_neuron '(record {id = null; command=opt variant {MakeProposal=record {url=""; title="NNS proposal Node Provider V2.1 Remuneration Model:";action=opt variant {Motion=record {motion_text="NNS proposal Node Provider V2.1 Remuneration Model:"}}; summary="# NNS proposal Node Provider V2.1 Remuneration Model:

This proposal describes the updated remuneration that is applicable to new Node Providers that implement nodes based on the Generation 2 Hardware specification, also called the V2.1 Remuneration Model. The remuneration is described in detail on this wiki page: https://wiki.internetcomputer.org/wiki/Node_Provider_Remuneration, and has been shared with the community in this forum thread: https://forum.dfinity.org/t/the-state-and-direction-of-decentralization-nodes-on-the-internet-computer/9170/193.

The V2.1 remuneration model takes into account several suggestions from the community:
- rewards should be based on costs for running node machines in a specific country instead of a more coarse-grained region, in particular in the Asia region.
- the multiplier covers the actual risk premium (business risk, project risk, country risk) for operating a node machine. The multiplier should not cover differences in operating costs for running a node machine. Like the first suggestion, this suggestion also implies to have specific remuneration per country instead of per region.
- incorporating decentralization goals, which should be to introduce Gen2 node machines in new countries (and to a limited extent in existing countries) until a certain number of node machines is reached. After this number of node machines is reached, there should be less incentive to add more node machines in that specific country as it would not add to the further decentralization of the IC-network. Hence, after a certain number of node machines has been reached in a country, the remuneration model should be updated to reflect this. This update applies only to the subsequently added nodes.

The following updates are proposed
- No entry for the region Asia will be used anymore. For the Asia region, specific country entries will be used.
- Specific entries for Hong Kong and India and other countries will be added to the remuneration table. In future, through community proposals, other country entries will be added as well.
- The multiplier is set to a value of 2 for all countries. In future, this might be updated through a new NNS proposal if the community wants that the risk premium for projects should be different for different countries (for example, if the risk of running in node machine in one specific country is higher than the risk of running in node machine in another country, this might validate a different multiplier).
- A limit is set to the number of node machines in new countries for which the remuneration applies. Once this is reached, the reduction coefficient for additional node machines will be adjusted to allow adding only one or two nodes for this country, similar to existing countries like U.S. and Switzerland. Currently, the limit of number of Gen2 node machines per country is set to 50, which allows for a node in a new country to be added to every existing subnet.

The following table shows the geography-dependent values and the monthly reward for the first node onboarded based on the Remuneration V2.1.

All prices are in XDR unless explicitly stated otherwise.

| Geography | Total cost over 4 years | Multiplier | Monthly reward first node | Reduction coefficient r |
| ----- | ----- | ----- | ----- | ----- |
| USA | 31034 | 2 | 1294 | 0.7 |
| US California | 37031 | 2 | 1543 | 0.7 |
| Canada | 37031 | 2 | 1543 | 0.7 |
| Germany | 36996 | 2 | 1542 | 0.7 |
| Switzerland | 36996 | 2 | 1542 | 0.7 |
| France | 36996 | 2 | 1542 | 0.7 |
| Belgium | 36996 | 2 | 1542 | 0.7 |
| Slovenia | 36996 | 2 | 1542 | 0.7 |
| Europe (other than the above) | 36996 | 2 | 1542 | 0.95 |
| Japan| 40508 | 2 | 1688 | 0.7 |
| Singapore | 40508 | 2 | 1688 | 0.7 |
| Hong Kong | 46141 | 2 | 1922 | 0.95 |
| India | 50377 | 2 | 2100 | 0.95 |
| South Africa | 55455 | 2 | 2310 | 0.95 |


The remuneration will be further updated and refined for new geographies, such as South America, Africa, and Australia as well. This is work in progress, and updates will be posted as a new proposal to be voted upon by the community."}}; neuron_id_or_subaccount=opt variant {NeuronId=record {id=40:nat64}}})'
WARN: The proposals identity is not stored securely. Do not use it to control a lot of cycles/ICP. Create a new identity with `dfx identity new` and use it in mainnet-facing commands with the `--identity` flag
(
  record {
    2_171_433_291 = opt variant {
      106_380_200 = record {
        1_389_388_560 = "Caller not authorized to propose.";
        3_790_638_545 = 3 : int32;
      }
    };
  },
)
+ set +x

Do you want to continue? [y/N]
```

# Gotchas

There may be some issues with the quoting, since the arguments are passed to the shell.
Be careful.

