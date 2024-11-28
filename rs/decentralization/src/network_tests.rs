#[cfg(test)]
mod tests {
    use ic_base_types::PrincipalId;
    use ic_management_types::{Node, NodeFeatures, Operator};

    use crate::network::{DecentralizedSubnet, ReplacementCandidate};

    #[test]
    fn test_empty_best_results() {
        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(PrincipalId::new_subnet_test_id(1), vec![]);
        let result = DecentralizedSubnet::choose_one_result(&[], &subnet.nodes, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_best_results_already_in_subnet() {
        let node1 = Node::new_test_node(1, NodeFeatures::default(), true);
        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(PrincipalId::new_subnet_test_id(1), vec![node1.clone()]);
        let best_results = vec![ReplacementCandidate::new_with_node_for_tests(node1.clone())];
        let all_nodes = vec![node1.clone()];

        let result = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);

        // Should return the node already in the subnet.
        assert_eq!(result.unwrap().node.principal, node1.principal);
    }

    #[test]
    fn test_none_of_best_results_in_current_nodes() {
        let node1 = Node::new_test_node(1, NodeFeatures::default(), true);
        let node2 = Node::new_test_node(2, NodeFeatures::default(), true);
        let node3 = Node::new_test_node(3, NodeFeatures::default(), true);

        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(PrincipalId::new_subnet_test_id(1), vec![node1.clone()]);

        let best_results = vec![
            ReplacementCandidate::new_with_node_for_tests(node2.clone()),
            ReplacementCandidate::new_with_node_for_tests(node3.clone()),
        ];

        let all_nodes = vec![node1.clone(), node2.clone(), node3.clone()];

        let result = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);
        assert!(result.is_some());
    }

    #[test]
    fn test_operator_percentage_tie_break() {
        let op1 = Operator {
            principal: PrincipalId::new_user_test_id(1),
            ..Default::default()
        };
        let op2 = Operator {
            principal: PrincipalId::new_user_test_id(2),
            ..Default::default()
        };
        let subnet_1_id = PrincipalId::new_subnet_test_id(1);

        // op1 and op2 each have 2 nodes
        // op2 should be chosen because it has more unassigned nodes
        let node1 = Node::new_test_node(1, NodeFeatures::default(), true)
            .with_operator(op1.clone())
            .with_subnet_id(subnet_1_id);
        let node2 = Node::new_test_node(2, NodeFeatures::default(), true).with_operator(op1.clone());
        let node3 = Node::new_test_node(3, NodeFeatures::default(), true).with_operator(op2.clone());
        let node4 = Node::new_test_node(4, NodeFeatures::default(), true).with_operator(op2.clone());

        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(subnet_1_id, vec![node1.clone()]);

        let best_results = vec![
            ReplacementCandidate::new_with_node_for_tests(node2.clone()),
            ReplacementCandidate::new_with_node_for_tests(node3.clone()),
            ReplacementCandidate::new_with_node_for_tests(node4.clone()),
        ];

        let all_nodes = vec![node1.clone(), node2.clone(), node3.clone(), node4.clone()];

        let result = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);

        // op2 has no nodes assigned to subnets, so node3 from op2 should be picked
        assert_eq!(result.unwrap().node.principal, node3.principal);
    }

    #[test]
    fn test_tie_break_with_equal_percentages() {
        // When operators have the same percentage, the function should fallback to deterministic random selection.

        let op1 = Operator {
            principal: PrincipalId::new_user_test_id(1),
            ..Default::default()
        };
        let op2 = Operator {
            principal: PrincipalId::new_user_test_id(2),
            ..Default::default()
        };

        let node1 = Node::new_test_node(1, NodeFeatures::default(), true)
            .with_operator(op1.clone())
            .with_subnet_id(PrincipalId::new_subnet_test_id(1));
        let node2 = Node::new_test_node(2, NodeFeatures::default(), true).with_operator(op1.clone());
        let node3 = Node::new_test_node(3, NodeFeatures::default(), true).with_operator(op2.clone());
        let node4 = Node::new_test_node(4, NodeFeatures::default(), true).with_operator(op2.clone());

        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(PrincipalId::new_subnet_test_id(1), vec![node1.clone()]);

        let best_results = vec![
            ReplacementCandidate::new_with_node_for_tests(node2.clone()),
            ReplacementCandidate::new_with_node_for_tests(node3.clone()),
            ReplacementCandidate::new_with_node_for_tests(node4.clone()),
        ];

        let all_nodes = vec![node1.clone(), node2.clone(), node3.clone(), node4.clone()];

        let result1 = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);
        let result2 = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);

        // Ensure deterministic selection with equal percentages.
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_deterministic_random_selection() {
        let node1 = Node::new_test_node(1, NodeFeatures::default(), true);
        let node2 = Node::new_test_node(2, NodeFeatures::default(), true);
        let node3 = Node::new_test_node(3, NodeFeatures::default(), true);

        let subnet = DecentralizedSubnet::new_with_subnet_id_and_nodes(PrincipalId::new_subnet_test_id(1), vec![node1.clone()]);

        let best_results = vec![
            ReplacementCandidate::new_with_node_for_tests(node2.clone()),
            ReplacementCandidate::new_with_node_for_tests(node3.clone()),
        ];

        let all_nodes = vec![node1.clone(), node2.clone(), node3.clone()];

        let result1 = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);
        let result2 = DecentralizedSubnet::choose_one_result(&best_results, &subnet.nodes, &all_nodes);

        // Deterministic behavior ensures the same result is chosen for the same input.
        assert_eq!(result1, result2);
    }
}
