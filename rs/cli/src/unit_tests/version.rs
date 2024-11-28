use indexmap::IndexMap;
use std::sync::{Arc, RwLock};

use futures::future::ok;
use ic_management_backend::{health::MockHealthStatusQuerier, lazy_git::MockLazyGit, lazy_registry::MockLazyRegistry, proposal::MockProposalAgent};
use ic_management_types::{Artifact, ArtifactReleases, Network};
use itertools::Itertools;

use crate::{
    artifact_downloader::MockArtifactDownloader,
    auth::Neuron,
    commands::ExecutableCommand,
    cordoned_feature_fetcher::MockCordonedFeatureFetcher,
    ctx::tests::get_mocked_ctx,
    ic_admin::{MockIcAdmin, ProposeCommand, ProposeOptions},
    runner::{format_regular_version_upgrade_summary, format_security_hotfix},
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
    registry.expect_subnets().returning(|| Box::pin(ok(Arc::new(IndexMap::new()))));
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
        Neuron::anonymous_neuron(),
        Arc::new(registry),
        Arc::new(ic_admin),
        Arc::new(git),
        Arc::new(proposal_agent),
        Arc::new(artifact_downloader),
        Arc::new(MockCordonedFeatureFetcher::new()),
        Arc::new(MockHealthStatusQuerier::new()),
    );

    for (name, expected_title, cmd) in [
        (
            "Regular version upgrade",
            "Elect new IC",
            crate::commands::version::revise::guest_os::GuestOs {
                version: "new_version".to_string(),
                release_tag: "rel_tag".to_string(),
                ignore_missing_urls: false,
                security_fix: false,
            },
        ),
        (
            "Security fix",
            "Security patch update",
            crate::commands::version::revise::guest_os::GuestOs {
                version: "new_version".to_string(),
                release_tag: "rel_tag".to_string(),
                ignore_missing_urls: false,
                security_fix: true,
            },
        ),
    ] {
        let resp = cmd.execute(ctx.clone()).await;
        assert!(resp.is_ok(), "Test {} failed, command finished with err: {:?}", name, resp.err().unwrap());

        let mut captured_cmd = captured_cmd.write().unwrap();
        let mut captured_opts = captured_opts.write().unwrap();
        assert!(
            captured_cmd.is_some() && captured_opts.is_some(),
            "Test {} failed, ic-admin not called but expected to be",
            name
        );
        let (artifact, args) = match captured_cmd.as_ref().unwrap() {
            ProposeCommand::ReviseElectedVersions { release_artifact, args } => (release_artifact, args),
            _ => panic!("Test {} captured an unexpected proposal command", name),
        };

        let opts = captured_opts.as_ref().unwrap();

        assert_eq!(*artifact, Artifact::GuestOs, "Test {} received an unexpected artifact", name);
        assert!(
            args.contains(&sha) && args.contains(&cmd.version),
            "Test {} arguments don't contain correct sha `{}` or version `{}`. Got [{}]",
            sha,
            cmd.version,
            name,
            args.iter().join(", ")
        );
        assert!(opts.title.as_ref().unwrap().starts_with(expected_title));
        assert_eq!(
            match cmd.security_fix {
                true => format_security_hotfix("https://forum.dfinity.org/t/123".to_string()),
                false => format_regular_version_upgrade_summary(
                    &cmd.version,
                    &Artifact::GuestOs,
                    &cmd.release_tag,
                    "https://forum.dfinity.org/t/123".to_string()
                )
                .unwrap(),
            },
            *opts.summary.as_ref().unwrap(),
        );

        // Prepare for next test
        *captured_cmd = None;
        *captured_opts = None;
    }
}
