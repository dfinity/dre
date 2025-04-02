use super::*;
use crate::reward_period::{DayEndNanos, NANOS_PER_DAY};
use ic_base_types::SubnetId;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardRates};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn provider_id(id: u64) -> PrincipalId {
    PrincipalId::new_user_test_id(id)
}

fn create_metrics_by_node() -> BTreeMap<NodeId, Vec<NodeMetricsDaily>> {
    let mut metrics_by_node = BTreeMap::new();
    metrics_by_node.insert(node_id(1), vec![NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(1), 0, 0)]);
    metrics_by_node
}

#[test]
fn test_empty_rewardable_nodes() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let result = validate_input(&reward_period, &BTreeMap::new(), &[]);

    assert_eq!(result, Err(RewardCalculationError::EmptyNodes));
}

#[test]
fn test_node_not_in_rewardables() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let metrics_by_node = create_metrics_by_node();

    let result = validate_input(&reward_period, &metrics_by_node, &[node_id(2)]);
    assert_eq!(result, Err(RewardCalculationError::NodeNotInRewardables(node_id(1))));
}

#[test]
fn test_metrics_out_of_range() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    let metrics_out_of_range = NodeMetricsDaily::new(0, subnet_id(1), 0, 0);
    metrics_by_node.get_mut(&node_id(1)).unwrap().push(metrics_out_of_range.clone());

    let result = validate_input(&reward_period, &metrics_by_node, &[node_id(1)]);

    assert_eq!(
        result,
        Err(RewardCalculationError::NodeMetricsOutOfRange {
            node_id: node_id(1),
            timestamp: metrics_out_of_range.ts.get(),
            reward_period,
        })
    );
}

#[test]
fn test_same_day_metrics_same_sub() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(1), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node, &[node_id(1)]);

    assert_eq!(result, Err(RewardCalculationError::DuplicateMetrics(node_id(1))));
}

#[test]
fn test_same_day_metrics_different_subs() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(2), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node, &[node_id(1)]);

    assert_eq!(result, Ok(()));
}

#[test]
fn test_node_provider_below_min_limit() {
    let node_provider_id = PrincipalId::new_anonymous();
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let rewards_table = NodeRewardsTable::default();
    let rewardables = vec![
        RewardableNode {
            node_id: PrincipalId::new_user_test_id(1).into(),
            node_provider_id,
            region: "region1".to_string(),
            node_type: "type1".to_string(),
        },
        RewardableNode {
            node_id: PrincipalId::new_user_test_id(2).into(),
            node_provider_id,
            region: "region1".to_string(),
            node_type: "type3.1".to_string(),
        },
    ];

    let rewards = calculate_rewards(&reward_period, &rewards_table, &BTreeMap::new(), &rewardables).unwrap();
    assert_eq!(rewards.results_per_provider.get(&node_provider_id).unwrap().rewards_total, dec!(2));
}

struct NPRInput {
    reward_period: RewardPeriod,
    rewards_table: NodeRewardsTable,
    metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    rewardables: Vec<RewardableNode>,
}

impl NPRInput {
    pub fn calculate_rewards(&self) -> RewardsCalculationResult {
        calculate_rewards(&self.reward_period, &self.rewards_table, &self.metrics_by_node, &self.rewardables).unwrap()
    }
}

struct NPRInputBuilder {
    reward_period: Option<RewardPeriod>,
    rewards_table: Option<NodeRewardsTable>,
    metrics_by_node: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    rewardables: Vec<RewardableNode>,
}

impl NPRInputBuilder {
    pub fn new() -> NPRInputBuilder {
        NPRInputBuilder {
            reward_period: None,
            rewards_table: None,
            metrics_by_node: BTreeMap::new(),
            rewardables: vec![],
        }
    }

    pub fn with_reward_period(&mut self, start: u64, end: u64) -> &mut NPRInputBuilder {
        self.reward_period = Some(RewardPeriod::new(start, end).unwrap());
        self
    }

    pub fn with_rewards_rates(&mut self, region: &str, node_types: Vec<&str>, rate: u64, coeff: u64) -> &mut NPRInputBuilder {
        let mut rates: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
        for node_type in node_types {
            rates.insert(
                node_type.to_string(),
                NodeRewardRate {
                    xdr_permyriad_per_node_per_month: rate,
                    reward_coefficient_percent: Some(coeff as i32),
                },
            );
        }
        self.rewards_table
            .get_or_insert_with(NodeRewardsTable::default)
            .table
            .insert(region.to_string(), NodeRewardRates { rates });
        self
    }

    pub fn with_node(&mut self, node_id: NodeId, node_provider_id: PrincipalId, region: &str, node_type: &str) -> &mut NPRInputBuilder {
        self.rewardables.push(RewardableNode {
            node_id,
            node_provider_id,
            region: region.to_string(),
            node_type: node_type.to_string(),
        });
        self
    }

    pub fn with_nodes(&mut self, node_ids: Vec<NodeId>, node_provider_id: PrincipalId, region: &str, node_type: &str) -> &mut NPRInputBuilder {
        for node_id in node_ids {
            self.with_node(node_id, node_provider_id, region, node_type);
        }
        self
    }

    pub fn with_node_metrics(&mut self, node_id: NodeId, ts_start: u64, failure_rates: Vec<Decimal>, subnet_id: SubnetId) -> &mut NPRInputBuilder {
        let daily_metrics: Vec<NodeMetricsDaily> = failure_rates
            .iter()
            .enumerate()
            .map(|(i, rate)| NodeMetricsDaily {
                ts: DayEndNanos::from(ts_start + i as u64 * NANOS_PER_DAY),
                subnet_assigned: subnet_id,
                num_blocks_proposed: 0,
                num_blocks_failed: 0,
                failure_rate: *rate,
            })
            .collect();

        self.metrics_by_node
            .entry(node_id)
            .and_modify(|metrics| metrics.extend(daily_metrics.iter().cloned().collect_vec()))
            .or_insert(daily_metrics);
        self
    }

    pub fn with_nodes_metrics(
        &mut self,
        node_ids: Vec<NodeId>,
        ts_start: u64,
        failure_rates: Vec<Decimal>,
        subnet_id: SubnetId,
    ) -> &mut NPRInputBuilder {
        for node_id in node_ids {
            self.with_node_metrics(node_id, ts_start, failure_rates.clone(), subnet_id);
        }
        self
    }

    pub fn build(&self) -> NPRInput {
        NPRInput {
            reward_period: self.reward_period.clone().unwrap(),
            rewards_table: self.rewards_table.clone().unwrap(),
            metrics_by_node: self.metrics_by_node.clone(),
            rewardables: self.rewardables.clone(),
        }
    }
}

#[test]
fn test_node_provider_rewards_one_assigned() {
    let subnet_1 = subnet_id(1);

    let np_1 = provider_id(1);
    let node_1 = node_id(1);
    let nodes_np_1 = vec![node_1, node_id(2), node_id(3), node_id(4), node_id(5)];

    let np_2 = provider_id(2);
    let nodes_np_2 = vec![node_id(6), node_id(7), node_id(8)];

    let input = NPRInputBuilder::new()
        .with_reward_period(0, 30 * NANOS_PER_DAY)
        .with_rewards_rates("A,B", vec!["type0", "type1", "type3"], 1000, 97)
        // Node Provider 1: node_1 assigned, rest unassigned
        .with_nodes(nodes_np_1, np_1, "A,B", "type1")
        .with_node_metrics(node_1, 0, vec![dec!(0.4), dec!(0.2), dec!(0.3), dec!(0.4)], subnet_1)
        // Node Provider 2: all assigned with 0 failure rate this for bringing the subnet failure rate to 0
        .with_nodes(nodes_np_2.clone(), np_2, "A,B", "type1")
        .with_nodes_metrics(nodes_np_2, 0, vec![dec!(0); 4], subnet_1)
        .build();

    let rewards = input.calculate_rewards();

    //     ┌─Node: 3jo2y-lqbaa-aaaaa-aaaap-2ai ─────────────────────┬─────────────────────────────┬───────────────────────────┬─────────────────────────────┬─────────────────────────────────┐
    //     │        Day (UTC)         │ Original Failure Rate [OFR] │       Subnet Assigned       │ Subnet Failure Rate [SFR] │ Relative Failure Rate [RFR] │ Extrapolated Failure Rate [EFR] │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │        01-01-1970        │             0.4             │ yndj2-3ybaa-aaaaa-aaaap-yai │             0             │             0.4             │                -                │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │        02-01-1970        │             0.2             │ yndj2-3ybaa-aaaaa-aaaap-yai │             0             │             0.2             │                -                │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │        03-01-1970        │             0.3             │ yndj2-3ybaa-aaaaa-aaaap-yai │             0             │             0.3             │                -                │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │        04-01-1970        │             0.4             │ yndj2-3ybaa-aaaaa-aaaap-yai │             0             │             0.4             │                -                │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │ 05-01-1970 to 31-01-1970 │             N/A             │             N/A             │            N/A            │             N/A             │              0.325              │
    //     └──────────────────────────┴─────────────────────────────┴─────────────────────────────┴───────────────────────────┴─────────────────────────────┴─────────────────────────────────┘
    //     ┌─Node: gfvbo-licaa-aaaaa-aaaap-2ai ─────────────────────┬─────────────────┬───────────────────────────┬─────────────────────────────┬─────────────────────────────────┐
    //     │        Day (UTC)         │ Original Failure Rate [OFR] │ Subnet Assigned │ Subnet Failure Rate [SFR] │ Relative Failure Rate [RFR] │ Extrapolated Failure Rate [EFR] │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │ 01-01-1970 to 31-01-1970 │             N/A             │       N/A       │            N/A            │             N/A             │              0.325              │
    //     └──────────────────────────┴─────────────────────────────┴─────────────────┴───────────────────────────┴─────────────────────────────┴─────────────────────────────────┘
    //     ┌─Node: 32uhy-eydaa-aaaaa-aaaap-2ai ─────────────────────┬─────────────────┬───────────────────────────┬─────────────────────────────┬─────────────────────────────────┐
    //     │        Day (UTC)         │ Original Failure Rate [OFR] │ Subnet Assigned │ Subnet Failure Rate [SFR] │ Relative Failure Rate [RFR] │ Extrapolated Failure Rate [EFR] │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │ 01-01-1970 to 31-01-1970 │             N/A             │       N/A       │            N/A            │             N/A             │              0.325              │
    //     └──────────────────────────┴─────────────────────────────┴─────────────────┴───────────────────────────┴─────────────────────────────┴─────────────────────────────────┘
    //     ┌─Node: hr2go-2qeaa-aaaaa-aaaap-2ai ─────────────────────┬─────────────────┬───────────────────────────┬─────────────────────────────┬─────────────────────────────────┐
    //     │        Day (UTC)         │ Original Failure Rate [OFR] │ Subnet Assigned │ Subnet Failure Rate [SFR] │ Relative Failure Rate [RFR] │ Extrapolated Failure Rate [EFR] │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │ 01-01-1970 to 31-01-1970 │             N/A             │       N/A       │            N/A            │             N/A             │              0.325              │
    //     └──────────────────────────┴─────────────────────────────┴─────────────────┴───────────────────────────┴─────────────────────────────┴─────────────────────────────────┘
    //     ┌─Node: 2o3ay-vafaa-aaaaa-aaaap-2ai ─────────────────────┬─────────────────┬───────────────────────────┬─────────────────────────────┬─────────────────────────────────┐
    //     │        Day (UTC)         │ Original Failure Rate [OFR] │ Subnet Assigned │ Subnet Failure Rate [SFR] │ Relative Failure Rate [RFR] │ Extrapolated Failure Rate [EFR] │
    //     ├──────────────────────────┼─────────────────────────────┼─────────────────┼───────────────────────────┼─────────────────────────────┼─────────────────────────────────┤
    //     │ 01-01-1970 to 31-01-1970 │             N/A             │       N/A       │            N/A            │             N/A             │              0.325              │
    //     └──────────────────────────┴─────────────────────────────┴─────────────────┴───────────────────────────┴─────────────────────────────┴─────────────────────────────────┘
    //     ┌─Legend─┬───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
    //     │ Steps  │ Description                                                                                               │
    //     │────────┼───────────────────────────────────────────────────────────────────────────────────────────────────────────┤
    //     │ Step 1 │ Average Relative Failure Rate [ARFR]: AVG(RFR(Assigned Days))                                             │
    //     │        │                                                                                                           │
    //     │ Step 2 │ Extrapolated Failure Rate [EFR]: AVG(ARFR)                                                                │
    //     │        │                                                                                                           │
    //     │ Step 3 │ Average Extrapolated Failure Rate [AEFR]: AVG(RFR(Assigned Days), EFR(Unassigned Days))                   │
    //     │        │                                                                                                           │
    //     │ Step 4 │ Rewards Reduction [RR]:                                                                                   │
    //     │        │     * For nodes with AEFR < 0.1, the rewards reduction is 0                                               │
    //     │        │     * For nodes with AEFR > 0.6, the rewards reduction is 0.8                                             │
    //     │        │     * For nodes with 0.1 <= AEFR <= 0.6, the rewards reduction is linearly interpolated between 0 and 0.8 │
    //     │        │                                                                                                           │
    //     │ Step 5 │ Performance Multiplier [PM]: 1 - RR                                                                       │
    //     │        │                                                                                                           │
    //     │ Step 6 │ Adjusted Rewards: Base Rewards * PM                                                                       │
    //     │        │                                                                                                           │
    //     │ Step 7 │ Rewards Total                                                                                             │
    //     └────────┴───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
    //     ┌─Nodes Computation───────────┬───────────┬─────────────┬──────────────┬────────┬────────┬────────┬────────┬────────┬────────────┬─────────────┐
    //     │           Node ID           │ Node Type │ Node Region │ Base Rewards │ Step 1 │ Step 2 │ Step 3 │ Step 4 │ Step 5 │   Step 6   │   Step 7    │
    //     │                             │           │             │              │        │        │        │        │        │            │             │
    //     ├─────────────────────────────┼───────────┼─────────────┼──────────────┼────────┼────────┼────────┼────────┼────────┼────────────┼─────────────┤
    //     │ 3jo2y-lqbaa-aaaaa-aaaap-2ai │   type1   │     A,B     │ 1000 myrXDR  │ 0.325  │ 0.325  │ 0.325  │ 0.360  │ 0.640  │ 640 myrXDR │ 3200 myrXDR │
    //     ├─────────────────────────────┼───────────┼─────────────┼──────────────┼────────┼        ┼────────┼────────┼────────┼────────────┼             ┤
    //     │ gfvbo-licaa-aaaaa-aaaap-2ai │   type1   │     A,B     │ 1000 myrXDR  │   -    │        │ 0.325  │ 0.360  │ 0.640  │ 640 myrXDR │             │
    //     ├─────────────────────────────┼───────────┼─────────────┼──────────────┼────────┼        ┼────────┼────────┼────────┼────────────┼             ┤
    //     │ 32uhy-eydaa-aaaaa-aaaap-2ai │   type1   │     A,B     │ 1000 myrXDR  │   -    │        │ 0.325  │ 0.360  │ 0.640  │ 640 myrXDR │             │
    //     ├─────────────────────────────┼───────────┼─────────────┼──────────────┼────────┼        ┼────────┼────────┼────────┼────────────┼             ┤
    //     │ hr2go-2qeaa-aaaaa-aaaap-2ai │   type1   │     A,B     │ 1000 myrXDR  │   -    │        │ 0.325  │ 0.360  │ 0.640  │ 640 myrXDR │             │
    //     ├─────────────────────────────┼───────────┼─────────────┼──────────────┼────────┼        ┼────────┼────────┼────────┼────────────┼             ┤
    //     │ 2o3ay-vafaa-aaaaa-aaaap-2ai │   type1   │     A,B     │ 1000 myrXDR  │   -    │        │ 0.325  │ 0.360  │ 0.640  │ 640 myrXDR │             │
    //     └─────────────────────────────┴───────────┴─────────────┴──────────────┴────────┴────────┴────────┴────────┴────────┴────────────┴─────────────┘
    assert_eq!(rewards.results_per_provider.get(&np_1).unwrap().rewards_total, dec!(3200));
}
