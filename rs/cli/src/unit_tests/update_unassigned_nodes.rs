use std::{str::FromStr, sync::Arc};

use crate::artifact_downloader::MockArtifactDownloader;
use crate::auth::Neuron;
use crate::commands::update_unassigned_nodes::UpdateUnassignedNodes;
use crate::confirm::ConfirmationModeOptions;
use crate::cordoned_feature_fetcher::MockCordonedFeatureFetcher;
use crate::exe::ExecutableCommand;
use crate::forum::ForumParameters;
use crate::ic_admin::MockIcAdmin;
use crate::submitter::SubmissionParameters;
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

fn mock_forum_parameters() -> ForumParameters {
    ForumParameters::disable_forum()
}

fn mock_confirmation_mode() -> ConfirmationModeOptions {
    ConfirmationModeOptions::for_unit_tests()
}

#[tokio::test]
async fn should_skip_update_same_version_nns_not_provided() {
    let mut ic_admin = MockIcAdmin::new();
    // In this test this function shouldn't be called
    ic_admin.expect_submit_proposal().never();
    let principal = PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("version".to_string())) }));

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
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: None,
        submission_parameters: SubmissionParameters {
            forum_parameters: mock_forum_parameters(),
            confirmation_mode: mock_confirmation_mode(),
        },
    };
    let response = cmd.execute(ctx).await;
    assert!(response.is_ok(), "Respose was: {:?}", response)
}

#[tokio::test]
async fn should_skip_update_same_version_nns_provided() {
    let mut ic_admin = MockIcAdmin::new();
    // In this test this function shouldn't be called
    ic_admin.expect_submit_proposal().never();

    let principal = PrincipalId::new_anonymous();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("version".to_string())) }));

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
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(principal.to_string()),
        submission_parameters: SubmissionParameters {
            forum_parameters: mock_forum_parameters(),
            confirmation_mode: mock_confirmation_mode(),
        },
    };

    assert!(cmd.execute(ctx).await.is_ok())
}

#[tokio::test]
async fn should_update_unassigned_nodes() {
    let mut ic_admin = MockIcAdmin::new();
    // Should be called since versions differ
    ic_admin.expect_simulate_proposal().once().returning(|_, _| Box::pin(async { Ok(()) }));
    ic_admin
        .expect_submit_proposal()
        .once()
        .returning(|_, _| Box::pin(async { Ok("Proposal 1".to_string()) }));

    let principal = PrincipalId::new_anonymous();

    let mut registry = registry_with_subnets(vec![Subnet {
        principal,
        replica_version: "version".to_string(),
        ..Default::default()
    }]);

    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(async { Ok(Arc::new("other".to_string())) }));

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
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(principal.to_string()),
        submission_parameters: SubmissionParameters {
            forum_parameters: mock_forum_parameters(),
            confirmation_mode: mock_confirmation_mode(),
        },
    };

    assert!(cmd.execute(ctx).await.is_ok())
}

#[tokio::test]
async fn should_fail_nns_not_found() {
    let mut ic_admin = MockIcAdmin::new();
    // Should not be called
    ic_admin.expect_submit_proposal().never();

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
    );

    let cmd = UpdateUnassignedNodes {
        nns_subnet_id: Some(other_principal.to_string()),
        submission_parameters: SubmissionParameters {
            forum_parameters: mock_forum_parameters(),
            confirmation_mode: mock_confirmation_mode(),
        },
    };

    assert!(cmd.execute(ctx).await.is_err())
}
