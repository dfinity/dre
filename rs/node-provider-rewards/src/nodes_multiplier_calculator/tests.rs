use super::*;
use crate::reward_period::NANOS_PER_DAY;
use ic_base_types::{PrincipalId, SubnetId};
use rust_decimal::Decimal;
use std::collections::HashMap;

fn subnet_id(index: u64) -> SubnetId {
    SubnetId::from(PrincipalId::new_subnet_test_id(index))
}

// Helper to create node IDs with optional override
fn node_id(index: u64) -> NodeId {
    NodeId::from(PrincipalId::new_node_test_id(index))
}

// Builder for daily failure rates
struct DailyFailureRateBuilder {
    rates: HashMap<NodeId, Vec<NodeDailyFailureRate>>,
    default_ts: TimestampNanos,
}

impl DailyFailureRateBuilder {
    fn new(default_ts: TimestampNanos) -> Self {
        Self {
            rates: HashMap::new(),
            default_ts,
        }
    }

    fn new_default(default_ts: TimestampNanos) -> Self {
        Self {
            rates: HashMap::new(),
            default_ts,
        }
    }

    // Add multiple entries from tuples (subnet_id, node_id, fr)
    fn add_entries(mut self, entries: Vec<(SubnetId, u64, Decimal)>) -> Self {
        for (subnet_id, node_id, fr) in entries {
            self = self.add_entry(subnet_id, node_id, fr);
        }
        self
    }

    // Add single entry with optional timestamp override
    fn add_entry(mut self, subnet_id: SubnetId, node_id: u64, fr: Decimal) -> Self {
        let node_id = NodeId::from(PrincipalId::new_node_test_id(node_id));
        self.rates.entry(node_id).or_default().push(NodeDailyFailureRate {
            ts: self.default_ts,
            value: NodeFailureRate::Defined {
                subnet_assigned: subnet_id,
                value: fr,
            },
        });
        self
    }

    fn build(self) -> HashMap<NodeId, Vec<NodeDailyFailureRate>> {
        self.rates
    }
}
// Helper function that returns the test input data.
fn get_test_input() -> (
    HashMap<NodeId, Vec<NodeDailyFailureRate>>,
    HashMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    Vec<(SubnetId, u64, Decimal)>,
) {
    // Define subnets and their failure rates.
    let subnet_1 = SubnetId::from(PrincipalId::new_user_test_id(100));
    let subnet_1_fr = dec!(0.2);

    let subnet_2 = SubnetId::from(PrincipalId::new_user_test_id(200));
    let subnet_2_fr = dec!(0.4);

    // Define input tuples: (subnet, node identifier, node failure rate).
    let input = vec![
        (subnet_1, 1, dec!(0.3)),
        (subnet_1, 2, dec!(0.5)),
        (subnet_1, 3, dec!(0.7)),
        (subnet_2, 4, dec!(0.2)),
        (subnet_2, 5, dec!(0.5)),
        (subnet_2, 6, dec!(0.6)),
    ];

    // Build the daily failure rates for each node.
    let mut daily_failure_rates: HashMap<NodeId, Vec<NodeDailyFailureRate>> = HashMap::new();
    for (subnet_id, node_id, fr) in input.iter().cloned() {
        let node_id = NodeId::from(PrincipalId::new_user_test_id(node_id));
        let daily_fr = NodeDailyFailureRate {
            ts: NANOS_PER_DAY,
            value: NodeFailureRate::Defined {
                subnet_assigned: subnet_id,
                value: fr,
            },
        };
        daily_failure_rates.entry(node_id).or_default().push(daily_fr);
    }

    // Build the subnet failure rates.
    let subnets_failure_rate = HashMap::from([
        (
            subnet_1,
            vec![SubnetDailyFailureRate {
                ts: NANOS_PER_DAY,
                value: subnet_1_fr,
            }],
        ),
        (
            subnet_2,
            vec![SubnetDailyFailureRate {
                ts: NANOS_PER_DAY,
                value: subnet_2_fr,
            }],
        ),
    ]);

    (daily_failure_rates, subnets_failure_rate, input)
}

#[test]
fn test_discount_failure_rate_for_node_1() {
    // Use the helper to create the common test data.
    let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();

    // Expected data for node 1.
    let node_id = NodeId::from(PrincipalId::new_user_test_id(1));
    let subnet_1 = input[0].0;
    let node_1_fr = dec!(0.3);
    let subnet_1_fr = dec!(0.2);

    // Initialize and process the calculator.
    let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
    calculator.discount_subnets_failure_rate();

    let node_1_result = calculator.nodes_failure_rates.get(&node_id).unwrap()[0].value.clone();

    assert_eq!(
        node_1_result,
        NodeFailureRate::DefinedRelative {
            subnet_assigned: subnet_1,
            original_failure_rate: node_1_fr,
            subnet_failure_rate: subnet_1_fr,
            value: node_1_fr - subnet_1_fr,
        }
    );
}

#[test]
fn test_discount_failure_rate_for_node_5() {
    let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();

    // Expected data for node 5.
    let node_id = NodeId::from(PrincipalId::new_user_test_id(5));
    let subnet_2 = input.iter().find(|(_, id, _)| *id == 5).unwrap().0;
    let node_5_fr = dec!(0.5);
    let subnet_2_fr = dec!(0.4);

    let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
    calculator.discount_subnets_failure_rate();

    let node_5_result = calculator.nodes_failure_rates.get(&node_id).unwrap()[0].value.clone();

    assert_eq!(
        node_5_result,
        NodeFailureRate::DefinedRelative {
            subnet_assigned: subnet_2,
            original_failure_rate: node_5_fr,
            subnet_failure_rate: subnet_2_fr,
            value: node_5_fr - subnet_2_fr,
        }
    );
}

#[test]
fn test_compute_failure_rate_extrapolated() {
    let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();
    let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
    calculator.discount_subnets_failure_rate();

    // Calculate the expected extrapolated failure rate.
    let total: Decimal = input
        .iter()
        .map(|(subnet, _, fr)| {
            let subnet_fr = if *subnet == SubnetId::from(PrincipalId::new_user_test_id(100)) {
                dec!(0.2)
            } else {
                dec!(0.4)
            };
            fr - subnet_fr
        })
        .sum();
    let expected_failure_rate_extrapolated = total / Decimal::from(input.len());

    assert_eq!(calculator.compute_failure_rate_extrapolated(), expected_failure_rate_extrapolated);
}

#[test]
fn test_replace_undefined_failure_rates() {
    let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();
    let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
    calculator.discount_subnets_failure_rate();

    // Calculate the expected extrapolated failure rate.
    let total: Decimal = input
        .iter()
        .map(|(subnet, _, fr)| {
            let subnet_fr = if *subnet == SubnetId::from(PrincipalId::new_user_test_id(100)) {
                dec!(0.2)
            } else {
                dec!(0.4)
            };
            fr - subnet_fr
        })
        .sum();
    let expected_failure_rate_extrapolated = total / Decimal::from(input.len());

    assert_eq!(calculator.compute_failure_rate_extrapolated(), expected_failure_rate_extrapolated);
}
