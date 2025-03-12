use super::*;
use crate::reward_period::NANOS_PER_DAY;
use ic_base_types::SubnetId;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardRates};

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn create_reward_period() -> RewardPeriod {
    RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap()
}

fn create_metrics_by_node() -> BTreeMap<NodeId, Vec<NodeDailyMetrics>> {
    let mut metrics_by_node = BTreeMap::new();
    metrics_by_node.insert(node_id(1), vec![NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(1), 0, 0)]);
    metrics_by_node
}

fn mocked_rewards_table() -> NodeRewardsTable {
    let mut rates_outer: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
    let mut rates_inner: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
    let mut table: BTreeMap<String, NodeRewardRates> = BTreeMap::new();

    let rate_outer = NodeRewardRate {
        xdr_permyriad_per_node_per_month: 1000,
        reward_coefficient_percent: Some(97),
    };

    let rate_inner = NodeRewardRate {
        xdr_permyriad_per_node_per_month: 1500,
        reward_coefficient_percent: Some(95),
    };

    rates_outer.insert("type0".to_string(), rate_outer);
    rates_outer.insert("type1".to_string(), rate_outer);
    rates_outer.insert("type3".to_string(), rate_outer);

    rates_inner.insert("type3.1".to_string(), rate_inner);

    table.insert("A,B,C".to_string(), NodeRewardRates { rates: rates_inner });
    table.insert("A,B".to_string(), NodeRewardRates { rates: rates_outer });

    NodeRewardsTable { table }
}

#[test]
fn test_empty_rewardable_nodes() {
    let reward_period = create_reward_period();
    let result = validate_input(&reward_period, &BTreeMap::new(), &vec![]);

    assert_eq!(result, Err(RewardCalculationError::EmptyNodes));
}

#[test]
fn test_node_not_in_rewardables() {
    let reward_period = create_reward_period();
    let metrics_by_node = create_metrics_by_node();

    let result = validate_input(&reward_period, &metrics_by_node, &vec![node_id(2)]);
    assert_eq!(result, Err(RewardCalculationError::NodeNotInRewardables(node_id(1))));
}

#[test]
fn test_metrics_out_of_range() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    let metrics_out_of_range = NodeDailyMetrics::new(0, subnet_id(1), 0, 0);
    metrics_by_node.get_mut(&node_id(1)).unwrap().push(metrics_out_of_range.clone());

    let result = validate_input(&reward_period, &metrics_by_node, &vec![node_id(1)]);

    assert_eq!(
        result,
        Err(RewardCalculationError::NodeMetricsOutOfRange {
            node_id: node_id(1),
            timestamp: *metrics_out_of_range.ts,
            reward_period,
        })
    );
}

#[test]
fn test_same_day_metrics_same_sub() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(1), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node, &vec![node_id(1)]);

    assert_eq!(result, Err(RewardCalculationError::DuplicateMetrics(node_id(1))));
}

#[test]
fn test_same_day_metrics_different_subs() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(2), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node, &vec![node_id(1)]);

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

    rewards
        .logs_per_node_provider
        .get(&node_provider_id)
        .unwrap()
        .iter()
        .for_each(|log| println!("{}", log));
    assert_eq!(*rewards.rewards_per_node_provider.get(&node_provider_id).unwrap(), 2u64);
}

#[test]
fn test_node_provider_rewards_one_assigned() {
    let rewards_table: NodeRewardsTable = mocked_rewards_table();
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();

    let rewardables: Vec<RewardableNode> = (1..=5)
        .map(|i| RewardableNode {
            node_id: PrincipalId::new_user_test_id(i).into(),
            node_provider_id: PrincipalId::new_anonymous(),
            region: "A,B".to_string(),
            node_type: "type1".to_string(),
        })
        .collect();

    let mut nodes_idiosyncratic_fr: HashMap<NodeId, Vec<Decimal>> = HashMap::new();
    nodes_idiosyncratic_fr.insert(
        PrincipalId::new_user_test_id(1).into(),
        vec![dec!(0.4), dec!(0.2), dec!(0.3), dec!(0.4)], // Avg. 0.325
    );

    let rewards = calculate_rewards(&reward_period, &rewards_table, &BTreeMap::new(), &rewardables).unwrap();

    // Compute Base Rewards For RegionNodeType
    //     - node_type: type1, region: A,B, coeff: 1, base_rewards: 1000, node_count: 5
    // Compute Unassigned Days Failure Rate
    //     - Avg. failure rate for node: 6fyp7-3ibaa-aaaaa-aaaap-4ai: avg(0.4,0.2,0.3,0.4) = 0.325
    //     - Unassigned days failure rate:: avg(0.325) = 0.325
    //     - Rewards reduction percent: (0.325 - 0.1) / (0.6 - 0.1) * 0.8 = 0.360
    //     - Reward multiplier fully unassigned nodes:: 1 - 0.360 = 0.640
    // Compute Rewards For Node | node_id=6fyp7-3ibaa-aaaaa-aaaap-4ai, node_type=type1, region=A,B
    //     - Base rewards XDRs: 1000
    //     - Node status: Assigned
    //     - Idiosyncratic daily failure rates : 0.4,0.2,0.3,0.4,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325
    //     - Failure rate average: avg(0.4,0.2,0.3,0.4,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325,0.325) = 0.325
    //     - Rewards reduction percent: (0.325 - 0.1) / (0.6 - 0.1) * 0.8 = 0.360
    //     - Reward Multiplier: 1 - 0.360 = 0.640
    //     - Rewards XDR for the node: 1000 * 0.640 = 640.000
    // Compute Rewards For Node | node_id=djduj-3qcaa-aaaaa-aaaap-4ai, node_type=type1, region=A,B
    //     - Base rewards XDRs: 1000
    //     - Node status: Unassigned
    //     - Rewards XDR for the node: 1000 * 0.640 = 640.000
    // Compute Rewards For Node | node_id=6wcs7-uadaa-aaaaa-aaaap-4ai, node_type=type1, region=A,B
    //     - Base rewards XDRs: 1000
    //     - Node status: Unassigned
    //     - Rewards XDR for the node: 1000 * 0.640 = 640.000
    // Compute Rewards For Node | node_id=c5mtj-kieaa-aaaaa-aaaap-4ai, node_type=type1, region=A,B
    //     - Base rewards XDRs: 1000
    //     - Node status: Unassigned
    //     - Rewards XDR for the node: 1000 * 0.640 = 640.000
    // Compute Rewards For Node | node_id=7cnv7-fyfaa-aaaaa-aaaap-4ai, node_type=type1, region=A,B
    //     - Base rewards XDRs: 1000
    //     - Node status: Unassigned
    //     - Rewards XDR for the node: 1000 * 0.640 = 640.000
    //     - Compute total permyriad XDR: sum(640.000,640.000,640.000,640.000,640.000) = 3200.000
    //     - Compute total permyriad XDR no performance penalty: sum(1000,1000,1000,1000,1000) = 5000
    // Total rewards XDR permyriad: 3200.000
    // Total rewards XDR permyriad not adjusted: 5000
    assert_eq!(rewards.xdr_permyriad, 3200);
    assert_eq!(rewards.xdr_permyriad_no_reduction, 5000);
}
