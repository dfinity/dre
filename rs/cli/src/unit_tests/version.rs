use std::{collections::BTreeMap, sync::Arc};

use decentralization::network::Identifies;
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
    let mut captured_cmd: Option<ProposeCommand> = None;
    let mut captured_opts: Option<ProposeOptions> = None;
    ic_admin.expect_propose_run().returning(|cmd, opts| {
        captured_cmd = Some(cmd.clone());
        captured_opts = Some(captured_opts.clone())
    });

    let mut git = MockLazyGit::new();
    git.expect_guestos_releases()
        .returning(|| Ok(Arc::new(ArtifactReleases::new(ic_management_types::Artifact::GuestOs))));

    let mut registry = MockLazyRegistry::new();
    registry.expect_subnets().returning(|| Ok(Arc::new(BTreeMap::new())));

    let mut proposal_agent = MockProposalAgent::new();
    proposal_agent.expect_list_open_elect_replica_proposals().returning(|| Ok(vec![]));

    let download_urls = ["https://ver1.download.link", "https://ver1.alt.download.link"]
        .iter()
        .map(|s| s.to_string())
        .collect_vec();
    let sha = "sha_of_ver".to_string();
    let mut artifact_downloader = MockArtifactDownloader::new();
    artifact_downloader
        .expect_download_images_and_validate_sha256()
        .returning(|| (download_urls, sha));

    let ctx = get_mocked_ctx(
        Network::mainnet_unchecked().unwrap(),
        registry,
        ic_admin,
        git,
        proposal_agent,
        artifact_downloader,
    );

    let cmd = crate::commands::version::revise::guest_os::GuestOs {
        version: "new_version".to_string(),
        release_tag: "rel_tag".to_string(),
        force: false,
        security_fix: false,
    };

    let resp = cmd.execute(ctx).await;
    assert!(resp.is_ok());

    assert!(captured_cmd.is_some() && captured_opts.is_some());
    let (artifact, args) = match captured_cmd.unwrap() {
        ProposeCommand::ReviseElectedVersions { release_artifact, args } => (release_artifact, args),
        _ => panic!("Unexpected proposal command"),
    };

    let opts = captured_opts.unwrap();

    assert_eq!(artifact, Artifact::GuestOs);
    assert!(args.contains(&sha) && args.contains(&cmd.version));
    assert!(opts.title.unwrap().starts_with("Elect new IC"));
    assert!(opts
        .summary
        .unwrap()
        .eq(&format_regular_version_upgrade_summary(&cmd.version, &Artifact::GuestOs, &cmd.release_tag).unwrap()))
}
