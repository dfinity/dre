use clap::Args;
use log::info;
use regex::Regex;
use serde_json::Value;
use tokio::task::JoinHandle;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Upgrade {}

impl Upgrade {
    pub async fn run(&self) -> anyhow::Result<UpdateStatus> {
        let version = env!("CARGO_PKG_VERSION");
        tokio::task::spawn_blocking(move || Self::check_latest_release(&version, true)).await?
    }

    pub fn check(&self) -> JoinHandle<anyhow::Result<UpdateStatus>> {
        let version = env!("CARGO_PKG_VERSION");
        tokio::task::spawn_blocking(move || Self::check_latest_release(&version, false))
    }

    fn check_latest_release(curr_version: &str, proceed_with_upgrade: bool) -> anyhow::Result<UpdateStatus> {
        // Check for a new release once per day
        let update_check_path = dirs::cache_dir().expect("Failed to find a cache dir").join("dre_update_check");
        if let Ok(metadata) = std::fs::metadata(&update_check_path) {
            let last_check = metadata.modified().unwrap();
            let now = std::time::SystemTime::now();
            if now.duration_since(last_check).unwrap().as_secs() < 60 * 60 * 24 {
                return Ok(UpdateStatus::NoUpdate);
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

        let latest_release = match releases.first() {
            Some(v) => v,
            None => return Err(anyhow::anyhow!("No releases found")),
        };

        if latest_release.version.eq(current_version) {
            return Ok(UpdateStatus::NoUpdate);
        }

        if !proceed_with_upgrade {
            return Ok(UpdateStatus::NewVersion(latest_release.version.clone()));
        }

        info!("Binary not up to date. Updating to {}", latest_release.version);

        let asset = match latest_release.asset_for("dre", None) {
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

        Ok(UpdateStatus::Updated(latest_release.version.clone()))
    }
}

pub enum UpdateStatus {
    NoUpdate,
    NewVersion(String),
    Updated(String),
}

impl ExecutableCommand for Upgrade {
    fn require_neuron(&self) -> bool {
        false
    }

    fn require_registry(&self) -> bool {
        false
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
