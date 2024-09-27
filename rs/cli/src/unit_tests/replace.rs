use std::sync::Arc;

use decentralization::{
    nakamoto::NodeFeatures,
    network::{DecentralizedSubnet, Node, NodeFeaturePair},
    SubnetChangeResponse,
};
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_registry::MockLazyRegistry};
use ic_management_types::NodeFeature;
use ic_types::PrincipalId;
use indexmap::{Equivalent, IndexMap};
use itertools::Itertools;

use crate::{cordoned_feature_fetcher::MockCordonedFeatureFetcher, subnet_manager::SubnetManager};

fn node_principal(id: u64) -> PrincipalId {
    PrincipalId::new_node_test_id(id)
}

fn node(id: u64, dfinity_owned: bool, features: &[(NodeFeature, &str)]) -> Node {
    Node {
        id: node_principal(id),
        dfinity_owned,
        features: NodeFeatures {
            feature_map: {
                let mut map = IndexMap::new();

                features.iter().for_each(|(feature, value)| {
                    map.insert(feature.clone(), value.to_string());
                });

                // Insert mandatory features
                for feature in &[NodeFeature::NodeProvider, NodeFeature::DataCenter, NodeFeature::DataCenterOwner] {
                    map.insert(feature.clone(), "Some value".to_string());
                }

                map
            },
        },
    }
}

fn subnet(id: u64, nodes: &[Node]) -> DecentralizedSubnet {
    DecentralizedSubnet {
        id: PrincipalId::new_subnet_test_id(id),
        nodes: nodes.to_vec(),
        added_nodes_desc: vec![],
        removed_nodes_desc: vec![],
        comment: None,
        run_log: vec![],
    }
}

fn cordoned_feature(feature: NodeFeature, value: &str) -> NodeFeaturePair {
    NodeFeaturePair {
        feature,
        value: value.to_string(),
    }
}

fn test_pretty_format_response(response: &Result<SubnetChangeResponse, anyhow::Error>) -> String {
    match response {
        Ok(r) => format!(
            r#"Response was OK!
    Added nodes:
{},
    Removed nodes:
{},
    Feature diff:
{}
            "#,
            r.added_with_desc
                .iter()
                .map(|(id, desc)| format!("\t\t- principal: {}\n\t\t  desc: {}", id, desc))
                .join("\n"),
            r.removed_with_desc
                .iter()
                .map(|(id, desc)| format!("\t\t- principal: {}\n\t\t  desc: {}", id, desc))
                .join("\n"),
            r.feature_diff
                .iter()
                .map(|(feature, diff)| format!(
                    "\t\t- feature: {}\n{}",
                    feature,
                    diff.iter()
                        .map(|(value, (in_nodes, out_nodes))| format!("\t\t\t- value: {}, In: {}, Out: {}", value, in_nodes, out_nodes))
                        .join("\n")
                ))
                .join("\n")
        ),
        Err(r) => format!("Response was ERR: {}", r),
    }
}

fn pretty_print_node(node: &Node, num_ident: usize) -> String {
    format!(
        "{}- principal: {}\n{}  dfinity_owned: {}\n{}  features: [{}]",
        "\t".repeat(num_ident),
        node.id,
        "\t".repeat(num_ident),
        node.dfinity_owned,
        "\t".repeat(num_ident),
        node.features
            .feature_map
            .iter()
            .map(|(feature, value)| format!("({}, {})", feature, value))
            .join(", ")
    )
}

fn pretty_print_world(available_nodes: &Vec<Node>, subnet: &DecentralizedSubnet) -> String {
    format!(
        r#"Available nodes:
{}
Observed subnet:
{}"#,
        available_nodes.iter().map(|node| pretty_print_node(node, 1)).join("\n"),
        format!(
            "\t- id: {}\n\t- nodes:\n{}",
            subnet.id,
            subnet.nodes.iter().map(|node| pretty_print_node(node, 2)).join("\n")
        )
    )
}

#[test]
fn should_skip_cordoned_nodes() {
    // World setup
    let available_nodes = vec![
        node(1, true, &[(NodeFeature::Country, "Country 1"), (NodeFeature::City, "City 1")]),
        node(2, true, &[(NodeFeature::Country, "Country 1"), (NodeFeature::City, "City 2")]),
        node(3, true, &[(NodeFeature::Country, "Country 2"), (NodeFeature::City, "City 3")]),
        node(4, true, &[(NodeFeature::Country, "Country 2"), (NodeFeature::City, "City 4")]),
    ];

    let subnet = subnet(
        1,
        &[
            node(5, true, &[(NodeFeature::Country, "Country 1"), (NodeFeature::City, "City 1")]),
            node(6, true, &[(NodeFeature::Country, "Country 2"), (NodeFeature::City, "City 3")]),
        ],
    );

    // Services setup
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut registry = MockLazyRegistry::new();
    let available_nodes_clone = available_nodes.clone();
    registry.expect_available_nodes().returning(move || {
        Box::pin({
            let local_clone = available_nodes_clone.clone();
            async move { Ok(local_clone) }
        })
    });
    let subnet_clone = subnet.clone();
    registry.expect_get_nodes().returning(move |_| {
        Box::pin({
            let nodes = subnet_clone.nodes.clone();
            async { Ok(nodes) }
        })
    });
    let subnet_clone = subnet.clone();
    registry.expect_subnet().returning(move |_| {
        Box::pin({
            let local_clone = subnet_clone.clone();
            async { Ok(local_clone) }
        })
    });

    let mut health_client = MockHealthStatusQuerier::new();
    // All nodes in the world are healthy for this test
    let nodes_health = available_nodes
        .iter()
        .map(|n| n.id)
        .chain(subnet.nodes.iter().map(|n| n.id))
        .map(|node_id| (node_id, ic_management_types::HealthStatus::Healthy))
        .collect::<IndexMap<PrincipalId, ic_management_types::HealthStatus>>();
    health_client.expect_nodes().returning(move || {
        Box::pin({
            let local_nodes_healht = nodes_health.clone();
            async move { Ok(local_nodes_healht) }
        })
    });

    // Scenarios
    let scenarios = vec![
        (
            // No available nodes contain cordoned features.
            // All of them should be suitable for replacements.
            vec![
                cordoned_feature(NodeFeature::Country, "Random new country"),
                cordoned_feature(NodeFeature::City, "Random new city"),
            ],
            true,
        ),
        (
            // First two nodes from available pool must not
            // be selected for replacement. Also node 5 could
            // be replaced if it increases decentralization.
            vec![cordoned_feature(NodeFeature::Country, "Country 1")],
            true,
        ),
        (
            // Second and third nodes from available pool must
            // not be selected for replacement. Also node with
            // id 6 could be replaced if it increases decentralization
            vec![
                cordoned_feature(NodeFeature::City, "City 2"),
                cordoned_feature(NodeFeature::City, "City 3"),
            ],
            true,
        ),
        (
            // All available nodes are unavailable
            vec![
                cordoned_feature(NodeFeature::Country, "Country 1"),
                cordoned_feature(NodeFeature::Country, "Country 2"),
            ],
            false,
        ),
    ];

    let mut failed_scenarios = vec![];

    let registry = Arc::new(registry);
    let health_client = Arc::new(health_client);
    for (cordoned_features, should_succeed) in scenarios {
        let cordoned_features_clone = cordoned_features.clone();
        let mut cordoned_feature_fetcher = MockCordonedFeatureFetcher::new();
        cordoned_feature_fetcher.expect_fetch().returning(move || {
            Box::pin({
                let local_clone = cordoned_features_clone.to_vec();
                async move { Ok(local_clone) }
            })
        });
        let subnet_manager = SubnetManager::new(registry.clone(), Arc::new(cordoned_feature_fetcher), health_client.clone());

        // Act
        let response = runtime.block_on(
            subnet_manager
                .with_target(crate::subnet_manager::SubnetTarget::FromNodesIds(vec![subnet.nodes.first().unwrap().id]))
                .membership_replace(false, None, None, None, vec![], None),
        );

        // Assert
        if !should_succeed {
            if response.is_ok() {
                failed_scenarios.push((response, cordoned_features, "Expected outcome to have an error".to_string()));
            }
            // If it failed, don't check the exact error
            // assume it is the correct error. ATM this
            // is not ideal but since we use anyhow its
            // hard to test exact expected errors
            continue;
        }

        let response = response.unwrap();
        if response.removed_with_desc.is_empty() {
            failed_scenarios.push((Ok(response), cordoned_features, "Expected nodes to be removed".to_string()));
            continue;
        }

        if response.added_with_desc.is_empty() {
            failed_scenarios.push((Ok(response), cordoned_features, "Expected nodes to be added".to_string()));
            continue;
        }

        let mut failed_features = vec![];
        for (feature, value_diff) in response.feature_diff.iter() {
            let cordoned_values_for_feature = cordoned_features
                .iter()
                .filter(|c_feature_pair| c_feature_pair.feature.equivalent(feature))
                .map(|c_feature_pair| c_feature_pair.value.clone())
                .collect_vec();

            if cordoned_features.is_empty() {
                // This feature is not cordoned so doesn't have to
                // be validated
                continue;
            }

            let diff_for_cordoned_features = value_diff
                .iter()
                .filter(|(value, _)| cordoned_values_for_feature.contains(value))
                .collect_vec();

            if diff_for_cordoned_features.is_empty() {
                // Feature is cordoned but for a different value
                // Example cordoned:       `NodeFeature::City`, value: `City 1`
                // Found for this replace: `NodeFeature::City`, value: `City 2`
                continue;
            }
            let mut failed_for_feature = vec![];
            for (value, (_, in_nodes)) in diff_for_cordoned_features {
                if in_nodes.gt(&0) {
                    failed_for_feature.push((value, in_nodes));
                    continue;
                }
            }

            if !failed_for_feature.is_empty() {
                failed_features.push(format!(
                    "Feature {} has cordoned values but still found some nodes added:\n{}",
                    feature,
                    failed_for_feature
                        .iter()
                        .map(|(value, in_nodes)| format!("\tValue: {}, New nodes with that value: {}", value, in_nodes))
                        .join("\n")
                ));
            }
        }

        if !failed_features.is_empty() {
            failed_scenarios.push((
                Ok(response),
                cordoned_features,
                format!("All failed features:\n{}", failed_features.iter().join("\n")),
            ));
        }
    }

    assert!(
        failed_scenarios.is_empty(),
        r#"World state:
{}
Failed scenarios:
{}"#,
        pretty_print_world(&available_nodes, &subnet),
        failed_scenarios
            .iter()
            .map(|(outcome, cordoned_features, explaination)| format!(
                r#"Reason why it failed:
    {}
Cordoned features:
    [{}]
Test output:
{}"#,
                explaination,
                cordoned_features
                    .iter()
                    .map(|pair| format!("({}, {})", pair.feature.to_string(), pair.value.to_string()))
                    .join(", "),
                test_pretty_format_response(outcome)
            ))
            .join("\n############")
    )
}
