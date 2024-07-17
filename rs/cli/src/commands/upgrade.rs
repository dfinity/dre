use clap::Args;
use log::info;
use regex::Regex;
use serde_json::Value;
use tokio::task::JoinHandle;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Upgrade {
    /// Version to which the tool should be upgraded, if omitted
    /// the latest version will be used
    #[clap(long, short)]
    version: Option<String>,
}

impl Upgrade {
    pub fn new() -> Self {
        Self { version: None }
    }

    pub async fn run(&self) -> anyhow::Result<UpdateStatus> {
        let version = self.version.clone();
        tokio::task::spawn_blocking(move || Self::check_latest_release(env!("CARGO_PKG_VERSION"), true, version)).await?
    }

    pub fn check(&self) -> JoinHandle<anyhow::Result<UpdateStatus>> {
        let version = env!("CARGO_PKG_VERSION");
        tokio::task::spawn_blocking(move || Self::check_latest_release(version, false, None))
    }

    fn check_latest_release(curr_version: &str, proceed_with_upgrade: bool, to_version: Option<String>) -> anyhow::Result<UpdateStatus> {
        // If the user called `Upgrade` don't check the metafile and
        // try the upgrade
        let update_check_path = dirs::cache_dir().expect("Failed to find a cache dir").join("dre_update_check");
        if !proceed_with_upgrade {
            // Check for a new release once per day
            if let Ok(metadata) = std::fs::metadata(&update_check_path) {
                let last_check = metadata.modified().unwrap();
                let now = std::time::SystemTime::now();
                if now.duration_since(last_check).unwrap().as_secs() < 60 * 60 * 24 {
                    return Ok(UpdateStatus::NoUpdate);
                }
            }
        }

        // ^                --> start of line
        // v?               --> optional 'v' char
        // (\d+\.\d+\.\d+)  --> string in format '1.22.33'
        // (-([0-9a-f])+)   --> string in format '-12345af' (optional)
        let re_version = Regex::new(r"^v?(\d+\.\d+\.\d+)(-([0-9a-f])+(\-dirty)?)?$").unwrap();
        let current_version = match re_version.captures(curr_version) {
            Some(cap) => cap.get(1).unwrap().as_str(),
            None => return Err(anyhow::anyhow!("Version '{}' doesn't follow expected naming", curr_version)),
        };

        let maybe_configured_backend = self_update::backends::github::ReleaseList::configure()
            .repo_owner("dfinity")
            .repo_name("dre")
            .build()
            .map_err(|e| anyhow::anyhow!("Configuring backend failed: {:?}", e))?;

        // Touch update check file
        std::fs::write(&update_check_path, "").map_err(|e| anyhow::anyhow!("Couldn't touch update check file: {:?}", e))?;

        let releases = maybe_configured_backend
            .fetch()
            .map_err(|e| anyhow::anyhow!("Fetching releases failed: {:?}", e))?;

        let release = match to_version {
            Some(to_v) => releases
                .iter()
                .find(|rel| PartialEq::eq(&rel.version, &to_v))
                .ok_or(anyhow::anyhow!("Release {} not found", to_v))?,
            None => releases.first().ok_or(anyhow::anyhow!("No releases found"))?,
        };

        if PartialEq::eq(&release.version, current_version) {
            return Ok(UpdateStatus::NoUpdate);
        }

        if !proceed_with_upgrade {
            return Ok(UpdateStatus::NewVersion(release.version.clone()));
        }

        let triple = match std::env::consts::OS {
            "linux" => "x86_64-unknown-linux",
            "macos" => "x86_64-apple-darwin",
            s => {
                return Err(anyhow::anyhow!(
                    "{} is not currently not supported for automatic upgrades. Try building the code from source",
                    s
                ))
            }
        };

        info!("Binary not up to date. Updating to {}", release.version);
        info!("Release: {:?}", release);

        let asset = match release.asset_for(&format!("dre-{}", triple), None) {
            Some(asset) => asset,
            None => return Err(anyhow::anyhow!("No assets found for release")),
        };

        let tmp_dir = tempfile::Builder::new()
            .prefix("self_update")
            .tempdir_in(::std::env::current_dir().unwrap())
            .map_err(|e| anyhow::anyhow!("Couldn't create temp dir: {:?}", e))?;

        let new_dre_path = tmp_dir.path().join(&asset.name);
        let asset_path = tmp_dir.path().join("asset");
        let asset_file = std::fs::File::create(&asset_path).map_err(|e| anyhow::anyhow!("Couldn't create file: {:?}", e))?;
        let new_dre_file = std::fs::File::create(&new_dre_path).map_err(|e| anyhow::anyhow!("Couldn't create file: {:?}", e))?;

        self_update::Download::from_url(&asset.download_url)
            .show_progress(true)
            .download_to(&asset_file)
            .map_err(|e| anyhow::anyhow!("Couldn't download asset: {:?}", e))?;

        info!("Asset downloaded successfully");

        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(&asset_path).unwrap()).map_err(|e| anyhow::anyhow!("Couldn't open asset: {:?}", e))?;

        let download_url = match value.get("browser_download_url") {
            Some(Value::String(d)) => d,
            Some(_) => return Err(anyhow::anyhow!("Unexpected type for url in asset")),
            None => return Err(anyhow::anyhow!("Download url not present in asset")),
        };

        self_update::Download::from_url(download_url)
            .show_progress(true)
            .download_to(&new_dre_file)
            .map_err(|e| anyhow::anyhow!("Couldn't download binary: {:?}", e))?;

        self_update::self_replace::self_replace(new_dre_path).map_err(|e| anyhow::anyhow!("Couldn't upgrade to the newest version: {:?}", e))?;

        // Since its possible to upgrade to an older version
        // remove the metafile so that the check will be run
        // with the new version again
        std::fs::remove_file(&update_check_path)?;

        Ok(UpdateStatus::Updated(release.version.clone()))
    }
}

pub enum UpdateStatus {
    NoUpdate,
    NewVersion(String),
    Updated(String),
}

impl ExecutableCommand for Upgrade {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    async fn execute(&self, _ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
