use std::path::Path;
use std::sync::Arc;

use futures::future::ok;
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::Network;
use ic_registry_common_proto::pb::local_store::v1::{ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType};
use prost::Message;
use serial_test::serial;

use crate::{
    artifact_downloader::MockArtifactDownloader, auth::Neuron, commands::registry::Registry, cordoned_feature_fetcher::MockCordonedFeatureFetcher,
    ctx::tests::get_mocked_ctx, ic_admin::MockIcAdmin,
};

fn hex_version(v: u64) -> String {
    format!("{v:016x}")
}

fn write_version(base: &Path, version: u64, mutations: Vec<PbKeyMutation>) {
    let filename = format!("{}.pb", hex_version(version));
    let file_path = base.join(filename);
    fs_err::create_dir_all(file_path.parent().unwrap()).unwrap();
    let entry = PbChangelogEntry { key_mutations: mutations };
    fs_err::write(file_path, entry.encode_to_vec()).unwrap();
}

#[tokio::test]
#[serial]
async fn dump_versions_outputs_records_sorted() {
    // Arrange: write under the test fallback path used by implementation
    // Constrain lookup to only our test dir
    let base = std::path::PathBuf::from("/tmp/dre-test-store/local_registry/mainnet/t_dump_sorted");
    std::env::set_var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE", &base);

    write_version(
        &base,
        2,
        vec![PbKeyMutation {
            key: "k2".into(),
            value: vec![2, 2],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        1,
        vec![PbKeyMutation {
            key: "k1".into(),
            value: vec![],
            mutation_type: MutationType::Unset as i32,
        }],
    );

    // Mock context
    let mut ic_admin = MockIcAdmin::new();
    ic_admin.expect_simulate_proposal().returning(|_, _| Box::pin(async { Ok(()) }));
    let mut git = MockLazyGit::new();
    git.expect_guestos_releases().returning(|| {
        Box::pin(ok(Arc::new(ic_management_types::ArtifactReleases::new(
            ic_management_types::Artifact::GuestOs,
        ))))
    });
    let mut registry = MockLazyRegistry::new();
    registry.expect_subnets().returning(|| Box::pin(ok(Arc::new(indexmap::IndexMap::new()))));
    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(ok(Arc::new("some_ver".to_string()))));
    let mut proposal_agent = MockProposalAgent::new();
    proposal_agent
        .expect_list_open_elect_replica_proposals()
        .returning(|| Box::pin(ok(vec![])));
    let mut artifact_downloader = MockArtifactDownloader::new();
    artifact_downloader
        .expect_download_images_and_validate_sha256()
        .returning(|_, _, _| Box::pin(async { Ok((vec![], String::new())) }));

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
        Arc::new(artifact_downloader),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
    );

    // Act & Assert: query versions individually to avoid interference
    let cmd_v1 = Registry {
        output: None,
        filters: vec![],
        height: None,
        dump_versions: None,
    };
    let j1 = cmd_v1.dump_versions_json(ctx.clone()).await.unwrap();
    let a1 = j1.as_array().unwrap();
    assert_eq!(a1.len(), 1);
    assert_eq!(a1[0]["version"].as_u64().unwrap(), 1);
    assert_eq!(a1[0]["key"], "k1");

    let cmd_v2 = Registry {
        output: None,
        filters: vec![],
        height: None,
        dump_versions: Some(vec![-1]),
    };
    let j2 = cmd_v2.dump_versions_json(ctx).await.unwrap();
    let a2 = j2.as_array().unwrap();
    assert_eq!(a2.len(), 1);
    assert_eq!(a2[0]["version"].as_u64().unwrap(), 2);
    assert_eq!(a2[0]["key"], "k2");
}

#[tokio::test]
#[serial]
async fn list_versions_only_outputs_numbers() {
    let base = std::path::PathBuf::from("/tmp/dre-test-store/local_registry/mainnet/t_list_only");
    std::env::set_var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE", &base);
    write_version(
        &base,
        42,
        vec![PbKeyMutation {
            key: "k".into(),
            value: vec![1],
            mutation_type: MutationType::Set as i32,
        }],
    );

    let mut ic_admin = MockIcAdmin::new();
    ic_admin.expect_simulate_proposal().returning(|_, _| Box::pin(async { Ok(()) }));
    let mut git = MockLazyGit::new();
    git.expect_guestos_releases().returning(|| {
        Box::pin(ok(Arc::new(ic_management_types::ArtifactReleases::new(
            ic_management_types::Artifact::GuestOs,
        ))))
    });
    let mut registry = MockLazyRegistry::new();
    registry.expect_subnets().returning(|| Box::pin(ok(Arc::new(indexmap::IndexMap::new()))));
    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(ok(Arc::new("some_ver".to_string()))));
    let mut proposal_agent = MockProposalAgent::new();
    proposal_agent
        .expect_list_open_elect_replica_proposals()
        .returning(|| Box::pin(ok(vec![])));
    let mut artifact_downloader = MockArtifactDownloader::new();
    artifact_downloader
        .expect_download_images_and_validate_sha256()
        .returning(|_, _, _| Box::pin(async { Ok((vec![], String::new())) }));

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
        Arc::new(artifact_downloader),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
    );

    let cmd = Registry {
        output: None,
        filters: vec![],
        height: None,
        dump_versions: Some(vec![0, -1]),
    };
    let json = cmd.dump_versions_json(ctx).await.unwrap();
    let arr = json.as_array().unwrap();
    assert!(arr.iter().any(|e| e["version"] == 42));
}

#[tokio::test]
#[serial]
async fn dump_versions_rejects_reversed_range() {
    // Arrange: write under the test fallback path used by implementation
    let base = std::path::PathBuf::from("/tmp/dre-test-store/local_registry/mainnet/t_reversed_range");
    std::env::set_var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE", &base);

    // Create a few versions
    write_version(
        &base,
        10,
        vec![PbKeyMutation {
            key: "a".into(),
            value: vec![1],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        20,
        vec![PbKeyMutation {
            key: "b".into(),
            value: vec![2],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        30,
        vec![PbKeyMutation {
            key: "c".into(),
            value: vec![3],
            mutation_type: MutationType::Set as i32,
        }],
    );

    let mut ic_admin = MockIcAdmin::new();
    ic_admin.expect_simulate_proposal().returning(|_, _| Box::pin(async { Ok(()) }));
    let mut git = MockLazyGit::new();
    git.expect_guestos_releases().returning(|| {
        Box::pin(ok(Arc::new(ic_management_types::ArtifactReleases::new(
            ic_management_types::Artifact::GuestOs,
        ))))
    });
    let mut registry = MockLazyRegistry::new();
    registry.expect_subnets().returning(|| Box::pin(ok(Arc::new(indexmap::IndexMap::new()))));
    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(ok(Arc::new("some_ver".to_string()))));
    let mut proposal_agent = MockProposalAgent::new();
    proposal_agent
        .expect_list_open_elect_replica_proposals()
        .returning(|| Box::pin(ok(vec![])));
    let mut artifact_downloader = MockArtifactDownloader::new();
    artifact_downloader
        .expect_download_images_and_validate_sha256()
        .returning(|_, _, _| Box::pin(async { Ok((vec![], String::new())) }));

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
        Arc::new(artifact_downloader),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
    );

    // Valid negative range: last 2 (end-exclusive)
    let ok_cmd = Registry {
        output: None,
        filters: vec![],
        height: None,
        dump_versions: Some(vec![-2, None.unwrap_or_default()]),
    };
    let ok_json = ok_cmd.dump_versions_json(ctx.clone()).await.unwrap();
    let ok_arr = ok_json.as_array().unwrap();
    assert!(ok_arr
        .iter()
        .all(|e| e["version"].as_u64().unwrap() == 20 || e["version"].as_u64().unwrap() == 30));

    // Reversed negative range should yield empty
    let bad_cmd = Registry {
        output: None,
        filters: vec![],
        height: None,
        dump_versions: Some(vec![-1, -5]),
    };
    let empty = bad_cmd.dump_versions_json(ctx).await.unwrap();
    let empty_arr = empty.as_array().unwrap();
    assert!(empty_arr.is_empty(), "expected empty result for reversed range [-1, -5]");
}
