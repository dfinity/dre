use std::{str::FromStr, sync::Arc};

use crate::artifact_downloader::MockArtifactDownloader;
use crate::auth::Neuron;
use crate::commands::{update_unassigned_nodes::UpdateUnassignedNodes, ExecutableCommand};
use crate::cordoned_feature_fetcher::MockCordonedFeatureFetcher;
use crate::discourse_client::MockDiscourseClient;
use crate::ic_admin::MockIcAdmin;
use ic_management_backend::health::MockHealthStatusQuerier;
use ic_management_backend::{lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::{Network, Subnet};
use ic_types::PrincipalId;

use crate::ctx::tests::get_mocked_ctx;

fn registry_with_subnets(subnets: Vec<Subnet>) -> MockLazyRegistry {
    let mut registry = MockLazyRegistry::new();

    registry.expect_subnets().returning(move || {
        Box::pin({
            let subnets = subnets.clone();
            async move { Ok(Arc::new(subnets.into_iter().map(|s| (s.principal, s.clone())).collect())) }
        })
    });

    registry
}

#[tokio::test]
async fn should_skip_update_same_version_nns_not_provided() {
    let mut ic_admin = MockIcAdmin::new();
    let principal = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("version".to_string())) }));

    // In this test this function shouldn't be called
    ic_admin.expect_propose_run().never();

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(MockIcAdmin::new()),
        Arc::new(MockLazyGit::new()),
        Arc::new(MockProposalAgent::new()),
        Arc::new(MockArtifactDownloader::new()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Arc::new(MockDiscourseClient::new()),
    );

    let cmd = UpdateUnassignedNodes { nns_subnet_id: None };
    let response = cmd.execute(ctx).await;
    assert!(response.is_ok(), "Respose was: {:?}", response)
}

#[tokio::test]
async fn should_skip_update_same_version_nns_provided() {
    let mut ic_admin = MockIcAdmin::new();

    let principal = PrincipalId::new_anonymous();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("version".to_string())) }));

    // In this test this function shouldn't be called
    ic_admin.expect_propose_run().never();

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(MockLazyGit::new()),
        Arc::new(MockProposalAgent::new()),
        Arc::new(MockArtifactDownloader::new()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Arc::new(MockDiscourseClient::new()),
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(principal.to_string()),
    };

    assert!(cmd.execute(ctx).await.is_ok())
}

#[tokio::test]
async fn should_update_unassigned_nodes() {
    let mut ic_admin = MockIcAdmin::new();

    let principal = PrincipalId::new_anonymous();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("other".to_string())) }));

    // Should be called since versions differ
    ic_admin
        .expect_propose_run()
        .once()
        .returning(|_, _| Box::pin(async { Ok("Proposal 1".to_string()) }));

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(MockLazyGit::new()),
        Arc::new(MockProposalAgent::new()),
        Arc::new(MockArtifactDownloader::new()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Arc::new(MockDiscourseClient::new()),
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(principal.to_string()),
    };

    assert!(cmd.execute(ctx).await.is_ok())
}

#[tokio::test]
async fn should_fail_nns_not_found() {
    let mut ic_admin = MockIcAdmin::new();

    let principal = PrincipalId::new_subnet_test_id(0);
    let other_principal = PrincipalId::new_subnet_test_id(1);

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("other".to_string())) }));

    // Should not be called
    ic_admin.expect_propose_run().never();

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(MockLazyGit::new()),
        Arc::new(MockProposalAgent::new()),
        Arc::new(MockArtifactDownloader::new()),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
        Arc::new(MockDiscourseClient::new()),
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(other_principal.to_string()),
    };

    assert!(cmd.execute(ctx).await.is_err())
}
