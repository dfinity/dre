use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use futures::future::ok;
use ic_management_backend::{lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::{Artifact, ArtifactReleases, Network};
use itertools::Itertools;

use crate::{
    artifact_downloader::MockArtifactDownloader,
    commands::ExecutableCommand,
    ctx::tests::get_mocked_ctx,
    ic_admin::{MockIcAdmin, ProposeCommand, ProposeOptions},
    runner::format_regular_version_upgrade_summary,
};

#[tokio::test]
async fn guest_os_elect_version_tests() {
    let mut ic_admin = MockIcAdmin::new();
    let captured_cmd: Arc<RwLock<Option<ProposeCommand>>> = Arc::new(RwLock::new(None));
    let captured_opts: Arc<RwLock<Option<ProposeOptions>>> = Arc::new(RwLock::new(None));
    let captured_cmd_clone = captured_cmd.clone();
    let captured_opts_clone = captured_opts.clone();
    ic_admin.expect_propose_run().returning(move |cmd, opts| {
        *captured_cmd_clone.write().unwrap() = Some(cmd.clone());
        *captured_opts_clone.write().unwrap() = Some(opts.clone());
        Box::pin(ok("Proposal 123".to_string()))
    });

    let mut git = MockLazyGit::new();
    git.expect_guestos_releases()
        .returning(|| Box::pin(ok(Arc::new(ArtifactReleases::new(ic_management_types::Artifact::GuestOs)))));

    let mut registry = MockLazyRegistry::new();
    registry.expect_subnets().returning(|| Box::pin(ok(Arc::new(BTreeMap::new()))));
    registry
        .expect_unassigned_nodes_replica_version()
        .returning(|| Box::pin(ok(Arc::new("some_ver".to_string()))));

    let mut proposal_agent = MockProposalAgent::new();
    proposal_agent
        .expect_list_open_elect_replica_proposals()
        .returning(|| Box::pin(ok(vec![])));

    let download_urls = ["https://ver1.download.link", "https://ver1.alt.download.link"]
        .iter()
        .map(|s| s.to_string())
        .collect_vec();
    let downloads_urls_clone = download_urls.clone();
    let sha = "sha_of_ver".to_string();
    let sha_clone = sha.clone();
    let mut artifact_downloader = MockArtifactDownloader::new();
    artifact_downloader
        .expect_download_images_and_validate_sha256()
        .returning(move |_, _, _| {
            Box::pin({
                let sha_clone = sha_clone.clone();
                let downloads_urls_clone = downloads_urls_clone.clone();
                async move { Ok((downloads_urls_clone, sha_clone)) }
            })
        });

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
        Arc::new(artifact_downloader),
    );

    let cmd = crate::commands::version::revise::guest_os::GuestOs {
        version: "new_version".to_string(),
        release_tag: "rel_tag".to_string(),
        force: false,
        security_fix: false,
    };

    let resp = cmd.execute(ctx).await;
    assert!(resp.is_ok());

    let captured_cmd = captured_cmd.read().unwrap();
    let captured_opts = captured_opts.read().unwrap();
    assert!(captured_cmd.is_some() && captured_opts.is_some());
    let (artifact, args) = match captured_cmd.as_ref().unwrap() {
        ProposeCommand::ReviseElectedVersions { release_artifact, args } => (release_artifact, args),
        _ => panic!("Unexpected proposal command"),
    };

    let opts = captured_opts.as_ref().unwrap();

    assert_eq!(*artifact, Artifact::GuestOs);
    assert!(args.contains(&sha) && args.contains(&cmd.version));
    assert!(opts.title.as_ref().unwrap().starts_with("Elect new IC"));
    assert_eq!(
        format_regular_version_upgrade_summary(&cmd.version, &Artifact::GuestOs, &cmd.release_tag).unwrap(),
        *opts.summary.as_ref().unwrap(),
    )
}
