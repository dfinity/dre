use std::path::Path;
use std::sync::Arc;

use futures::future::ok;
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::Network;
use ic_registry_common_proto::pb::local_store::v1::{ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType};
use prost::Message;
use serial_test::serial;

use crate::exe::ExecutableCommand;

use crate::{
    artifact_downloader::MockArtifactDownloader, auth::Neuron, commands::registry::history::History,
    cordoned_feature_fetcher::MockCordonedFeatureFetcher, ctx::tests::get_mocked_ctx, ic_admin::MockIcAdmin,
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
async fn history_outputs_records_sorted() {
    // Arrange: write under the test fallback path used by implementation
    // Constrain lookup to only our test dir
    let base = std::path::PathBuf::from("/tmp/dre-test-store/local_registry/mainnet/t_history_sorted");
    unsafe {
        std::env::set_var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE", &base);
    }

    write_version(
        &base,
        42,
        vec![PbKeyMutation {
            key: "k2".into(),
            value: vec![2, 2],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        41,
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

    // Test 1: All versions (empty range)
    let output_file1 = std::path::PathBuf::from("/tmp/test_history_output1.json");
    let cmd_v1: History = History {
        version_1: Some(-2),
        version_2: None,
        output: Some(output_file1.clone()),
        filter: vec![],
    };
    cmd_v1.execute(ctx.clone()).await.unwrap();
    let content1 = std::fs::read_to_string(&output_file1).unwrap();
    let j1: serde_json::Value = serde_json::from_str(&content1).unwrap();
    let a1 = j1.as_array().unwrap();
    assert_eq!(a1.len(), 2);
    assert_eq!(a1[0]["version"].as_u64().unwrap(), 41);
    assert_eq!(a1[0]["key"], "k1");
    assert_eq!(a1[1]["version"].as_u64().unwrap(), 42);
    assert_eq!(a1[1]["key"], "k2");
    std::fs::remove_file(&output_file1).ok();

    // Test 2: Last version only
    let output_file2 = std::path::PathBuf::from("/tmp/test_history_output2.json");
    let cmd_v2 = History {
        version_1: Some(-1),
        version_2: None,
        output: Some(output_file1.clone()),
        filter: vec![],
    };
    cmd_v2.execute(ctx).await.unwrap();
    let content2 = std::fs::read_to_string(&output_file2).unwrap();
    let j2: serde_json::Value = serde_json::from_str(&content2).unwrap();
    let a2 = j2.as_array().unwrap();
    assert_eq!(a2.len(), 1);
    assert_eq!(a2[0]["version"].as_u64().unwrap(), 42);
    assert_eq!(a2[0]["key"], "k2");
    std::fs::remove_file(&output_file2).ok();
}

#[tokio::test]
#[serial]
async fn list_versions_only_outputs_numbers() {
    let base = std::path::PathBuf::from("/tmp/dre-test-store/local_registry/mainnet/t_list_only");
    unsafe {
        std::env::set_var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE", &base);
    }
    write_version(
        &base,
        1,
        vec![PbKeyMutation {
            key: "k".into(),
            value: vec![1],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        2,
        vec![PbKeyMutation {
            key: "k".into(),
            value: vec![2],
            mutation_type: MutationType::Set as i32,
        }],
    );
    write_version(
        &base,
        3,
        vec![PbKeyMutation {
            key: "k".into(),
            value: vec![2],
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

    let output_file = std::path::PathBuf::from("/tmp/test_list_versions_output.json");
    let cmd = History {
        version_1: Some(2),
        version_2: Some(2),
        output: Some(output_file.clone()),
        filter: vec![],
    };
    cmd.execute(ctx).await.unwrap();
    let content = std::fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let arr = json.as_array().unwrap();
    println!("arr: {:?}", arr);
    assert!(arr.iter().any(|e| e["version"] == 1));
    std::fs::remove_file(&output_file).ok();
}
