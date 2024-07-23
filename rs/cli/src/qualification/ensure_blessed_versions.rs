use backon::{ExponentialBuilder, Retryable};
use comfy_table::CellAlignment;
use itertools::Itertools;

use crate::{
    ctx::DreContext,
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::Step,
};

use super::{comfy_table_util::Table, print_table};

pub struct EnsureBlessedRevisions {
    pub version: String,
}

impl Step for EnsureBlessedRevisions {
    fn help(&self) -> String {
        "This step runs the check to determine if all versions are blessed".to_string()
    }

    fn name(&self) -> String {
        "ensure_blessed_revision".to_string()
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;
        let blessed_versions = registry.elected_guestos()?;

        if blessed_versions.contains(&self.version) {
            return Ok(());
        }
        let sha = fetch_shasum_for_disk_img(&self.version).await?;

        // Place proposal
        let place_proposal = || async {
            ctx.ic_admin()
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
                                "http://download.proxy-global.dfinity.network:8080/ic/{}/guest-os/update-img/update-img.tar.gz",
                                &self.version
                            ),
                        ],
                    },
                    ProposeOptions {
                        title: Some(format!("Blessing version: {}", &self.version)),
                        summary: Some("Some updates".to_string()),
                        ..Default::default()
                    },
                )
                .await
        };

        place_proposal.retry(&ExponentialBuilder::default()).await?;

        registry.sync_with_nns().await?;
        let blessed_versions = registry.elected_guestos()?;

        let table = Table::new()
            .with_columns(&[("Blessed versions", CellAlignment::Center)])
            .with_rows(blessed_versions.iter().map(|ver| vec![ver.to_string()]).collect_vec())
            .to_table();

        print_table(table);
        Ok(())
    }
}
const TAR_EXTENSION: &str = "update-img.tar.gz";
async fn fetch_shasum_for_disk_img(version: &str) -> anyhow::Result<String> {
    let url = format!(
        "http://download.proxy-global.dfinity.network:8080/ic/{}/guest-os/update-img/SHA256SUMS",
        version
    );
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
