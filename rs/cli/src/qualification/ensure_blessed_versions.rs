use itertools::Itertools;

use crate::{
    ic_admin::{ProposeCommand, ProposeOptions},
    qualification::Step,
};

use super::{
    print_table,
    tabular_util::{ColumnAlignment, Table},
    QualificationContext,
};

#[derive(Default)]
pub struct EnsureBlessedRevisions {}

impl Step for EnsureBlessedRevisions {
    fn help(&self) -> &'static str {
        "This step runs the check to determine if all versions are blessed"
    }

    fn name(&self) -> &'static str {
        "1a_ensure_blessed_revision"
    }

    async fn execute(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx.registry().await;
        let blessed_versions = registry.elected_guestos()?;

        if blessed_versions.contains(&ctx.to_version) {
            return Ok(());
        }
        let sha = fetch_shasum_for_disk_img(&ctx.to_version).await?;

        // Place proposal
        let ic_admin = ctx.dre_ctx.ic_admin();
        let output = ic_admin
            .propose_run(
                ProposeCommand::ReviseElectedVersions {
                    release_artifact: ic_management_types::Artifact::GuestOs,
                    args: vec![
                        "--replica-version-to-elect".to_string(),
                        ctx.to_version.clone(),
                        "--release-package-sha256-hex".to_string(),
                        sha,
                        "--release-package-urls".to_string(),
                        format!(
                            "http://download.proxy-global.dfinity.network:8080/ic/{}/guest-os/update-img/update-img.tar.gz",
                            &ctx.to_version
                        ),
                    ],
                },
                ProposeOptions {
                    title: Some(format!("Blessing version: {}", &ctx.to_version)),
                    summary: Some(format!("Some updates")),
                    ..Default::default()
                },
            )
            .await?;
        println!("{}", output);
        // Vote

        Ok(())
    }

    async fn print_status(&self, ctx: &QualificationContext) -> anyhow::Result<()> {
        let registry = ctx.dre_ctx.registry().await;
        let blessed_versions = registry.elected_guestos()?;

        let table = Table::new()
            .with_columns(&[("Blessed versions", ColumnAlignment::Middle)])
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
