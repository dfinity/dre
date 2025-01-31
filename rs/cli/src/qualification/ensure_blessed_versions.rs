use backon::{ExponentialBuilder, Retryable};
use comfy_table::CellAlignment;
use itertools::Itertools;

use crate::{
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::Step,
};

use super::{comfy_table_util::Table, util::StepCtx};

pub struct EnsureBlessedRevisions {
    pub version: String,
}

impl Step for EnsureBlessedRevisions {
    fn help(&self) -> String {
        format!("Check if version {} is blessed", self.version)
    }

    fn name(&self) -> String {
        "ensure_blessed_revision".to_string()
    }

    async fn execute(&self, ctx: &StepCtx) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx().registry().await;
        let blessed_versions = registry.elected_guestos().await?;

        if blessed_versions.contains(&self.version) {
            return Ok(());
        }
        let sha = fetch_shasum_for_disk_img(&self.version).await?;

        // Place proposal
        let place_proposal = || async {
            ctx.dre_ctx()
                .ic_admin()
                .await?
                .propose_run(
                    ProposeCommand::ReviseElectedVersions {
                        release_artifact: ic_management_types::Artifact::GuestOs,
                        args: vec![
                            "--replica-version-to-elect".to_string(),
                            self.version.clone(),
                            "--release-package-sha256-hex".to_string(),
                            sha.clone(),
                            "--release-package-urls".to_string(),
                            format!(
                                "https://download.dfinity.systems/ic/{}/guest-os/update-img/update-img.tar.zst",
                                &self.version
                            ),
                        ],
                    },
                    ProposeOptions {
                        title: Some(format!("Blessing version: {}", &self.version)),
                        summary: Some("Some updates".to_string()),
                        forum_post_link: None, // Qualification step, no forum post.
                        motivation: None,
                    },
                )
                .await
        };

        place_proposal.retry(ExponentialBuilder::default()).await?;

        registry.sync_with_nns().await?;
        let blessed_versions = registry.elected_guestos().await?;

        let table = Table::new()
            .with_columns(&[("Blessed versions", CellAlignment::Center)])
            .with_rows(blessed_versions.iter().map(|ver| vec![ver.to_string()]).collect_vec())
            .to_table();

        ctx.print_table(table);
        Ok(())
    }
}
const TAR_EXTENSION: &str = "update-img.tar.zst";
async fn fetch_shasum_for_disk_img(version: &str) -> anyhow::Result<String> {
    let url = format!("https://download.dfinity.systems/ic/{}/guest-os/update-img/SHA256SUMS", version);
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        panic!("Received non-success response status: {:?}", response.status())
    }

    Ok(String::from_utf8(response.bytes().await?.to_vec())?
        .lines()
        .find(|l| l.ends_with(TAR_EXTENSION))
        .ok_or(anyhow::anyhow!("Failed to find a hash ending with `{}` from: {}", &url, TAR_EXTENSION))?
        .split_whitespace()
        .next()
        .ok_or(anyhow::anyhow!("The format should contain whitespace"))?
        .to_string())
}
