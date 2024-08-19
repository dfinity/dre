use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use chrono::Utc;
use comfy_table::CellAlignment;
use flate2::bufread::GzDecoder;
use headless_chrome::{protocol::cdp::Page, Browser, LaunchOptionsBuilder};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use strum::{EnumIter, IntoEnumIterator};
use url::Url;

use crate::ctx::DreContext;

use super::comfy_table_util::Table;

pub const REQWEST_TIMEOUT: Duration = Duration::from_secs(30);
const IC_EXECUTABLES_DIR: &str = "ic-executables";

pub struct StepCtx {
    dre_ctx: DreContext,
    artifacts: Option<PathBuf>,
    log_path: Option<PathBuf>,
    client: Client,
    grafana_url: Option<String>,
}

impl StepCtx {
    pub fn new(dre_ctx: DreContext, artifacts: Option<PathBuf>, grafana_url: Option<String>) -> anyhow::Result<Self> {
        let artifacts_of_run = artifacts.as_ref().map(|t| {
            if let Err(e) = std::fs::create_dir_all(t) {
                panic!("Couldn't create dir {}: {:?}", t.display(), e)
            }
            t.clone()
        });
        Ok(Self {
            dre_ctx,
            log_path: artifacts_of_run.as_ref().map(|t| {
                let path = t.join("run.log");
                if let Err(e) = OpenOptions::new().write(true).truncate(true).create(true).open(&path) {
                    panic!("Couldn't create file {}: {:?}", path.display(), e)
                };
                path
            }),
            artifacts: artifacts_of_run,
            client: ClientBuilder::new().timeout(REQWEST_TIMEOUT).build()?,
            grafana_url,
        })
    }

    pub fn dre_ctx(&self) -> &DreContext {
        &self.dre_ctx
    }

    pub async fn download_canister(&self, canister: &str, version: &str) -> anyhow::Result<PathBuf> {
        let cache = dirs::cache_dir().ok_or(anyhow::anyhow!("Can't cache dir"))?.join(IC_EXECUTABLES_DIR);
        if !cache.exists() {
            std::fs::create_dir_all(&cache)?;
        }

        let artifact_path = cache.join(format!("{}/{}.{}", canister, canister, version));
        let artifact_dir = artifact_path.parent().unwrap();
        if !artifact_dir.exists() {
            std::fs::create_dir(artifact_dir)?;
        }

        let canister_path = PathBuf::from_str(&format!("{}.wasm", artifact_path.display())).map_err(|e| anyhow::anyhow!(e))?;

        if canister_path.exists() {
            self.print_text(format!("Canister `{}` data already present", canister));
            return Ok(canister_path);
        }

        let url = format!("https://download.dfinity.systems/ic/{}/canisters/{}.wasm.gz", version, canister);

        self.print_text(format!("Downloading: {}", url));
        let response = self.client.get(&url).send().await?.error_for_status()?.bytes().await?;
        let mut d = GzDecoder::new(&response[..]);
        let mut collector: Vec<u8> = vec![];
        let mut file = std::fs::File::create(&canister_path)?;
        d.read_to_end(&mut collector)?;

        file.write_all(&collector)?;
        self.print_text(format!("Downloaded: {}", &url));
        Ok(canister_path)
    }

    pub async fn download_executable(&self, executable: &str, version: &str) -> anyhow::Result<PathBuf> {
        let cache = dirs::cache_dir().ok_or(anyhow::anyhow!("Can't cache dir"))?.join(IC_EXECUTABLES_DIR);
        if !cache.exists() {
            std::fs::create_dir_all(&cache)?;
        }

        let exe_path = cache.join(format!("{}/{}.{}", executable, executable, version));
        let artifact_dir = exe_path.parent().unwrap();
        if !artifact_dir.exists() {
            std::fs::create_dir(artifact_dir)?;
        }

        if exe_path.exists() && exe_path.is_file() {
            let permissions = exe_path.metadata()?.permissions();
            let is_executable = permissions.mode() & 0o111 != 0;
            if is_executable {
                self.print_text(format!("Executable `{}` already present and executable", executable));
                return Ok(exe_path);
            }
        }

        let url = format!(
            "https://download.dfinity.systems/ic/{}/binaries/x86_64-{}/{}.gz",
            version,
            match std::env::consts::OS {
                "linux" => "linux",
                "macos" => "darwin",
                s => return Err(anyhow::anyhow!("Unsupported os: {}", s)),
            },
            executable
        );

        self.print_text(format!("Downloading: {}", url));
        let response = self.client.get(&url).send().await?.error_for_status()?.bytes().await?;
        let mut d = GzDecoder::new(&response[..]);
        let mut collector: Vec<u8> = vec![];
        let mut file = std::fs::File::create(&exe_path)?;
        d.read_to_end(&mut collector)?;

        file.write_all(&collector)?;
        self.print_text(format!("Downloaded: {}", &url));

        file.set_permissions(PermissionsExt::from_mode(0o774))?;
        self.print_text(format!("Created executable: {}", exe_path.display()));
        Ok(exe_path)
    }

    pub async fn print_subnet_versions(&self) -> anyhow::Result<()> {
        let registry = self.dre_ctx.registry().await;
        let subnets = registry.subnets().await?;

        let subnets = subnets.values();
        let unassigned = registry.unassigned_nodes_replica_version()?;
        let table = Table::new()
            .with_columns(&[
                ("Subnet type", CellAlignment::Left),
                ("Subnet Id", CellAlignment::Center),
                ("Version", CellAlignment::Center),
            ])
            .with_rows(
                subnets
                    .map(|s| {
                        vec![
                            match s.subnet_type {
                                SubnetType::Application => "application".to_string(),
                                SubnetType::System => "system".to_string(),
                                SubnetType::VerifiedApplication => "verified-app".to_string(),
                            },
                            s.principal.to_string(),
                            s.replica_version.clone(),
                        ]
                    })
                    .chain(vec![vec!["unassigned".to_string(), "unassigned".to_string(), unassigned.to_string()]])
                    .collect_vec(),
            )
            .to_table();

        self.print_table(table);

        Ok(())
    }

    pub fn print_text(&self, message: String) {
        self._print_with_time(message, false)
    }

    pub fn print_table(&self, table: comfy_table::Table) {
        self._print_with_time(format!("{}", table), true)
    }

    fn _print_with_time(&self, message: String, add_new_line: bool) {
        let current_time = Utc::now();
        let formatted = format!(
            "[{}]{}{}",
            current_time,
            match add_new_line {
                true => '\n',
                false => ' ',
            },
            message
        );

        if let Some(log_path) = &self.log_path {
            let mut file = match OpenOptions::new().append(true).open(log_path) {
                Ok(f) => f,
                Err(e) => panic!("Couldn't open file {}: {:?}", log_path.display(), e),
            };
            if let Err(e) = writeln!(file, "{}", formatted) {
                panic!("Couldn't append to file {}: {:?}", log_path.display(), e)
            }
        }

        println!("{}", formatted)
    }

    pub fn capture_progress_clock(
        &self,
        deployment_name: String,
        subnet: &PrincipalId,
        from: Option<i64>,
        to: Option<i64>,
        path_suffix: &str,
    ) -> anyhow::Result<()> {
        let (url, artifacts) = match (self.grafana_url.as_ref(), self.artifacts.as_ref()) {
            (Some(url), Some(artifacts)) => (url, artifacts),
            _ => return Ok(()),
        };

        let timestamp = match from {
            Some(t) => t.to_string(),
            None => Utc::now().timestamp().to_string(),
        };

        let browser = Browser::new(LaunchOptionsBuilder::default().devtools(false).window_size(Some((1920, 1080))).build()?)?;
        let tab = browser.new_tab()?;

        for panel in Panel::iter() {
            let mut url = Url::parse(url)?.join(panel.get_dashboard())?;
            url.set_query(Some(
                &[
                    ("var-ic", deployment_name.to_string()),
                    ("var-ic_subnet", subnet.to_string()),
                    (
                        "from",
                        match from {
                            Some(f) => (f * 1000).to_string(),
                            None => "now-1h".to_owned(),
                        },
                    ),
                    (
                        "to",
                        match to {
                            Some(t) => (t * 1000).to_string(),
                            None => "now".to_owned(),
                        },
                    ),
                    ("orgId", "1".to_owned()),
                    ("viewPanel", panel.into()),
                ]
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .join("&"),
            ));

            let destination = artifacts.join(format!("{}-{}-{}.png", panel.get_name(), path_suffix, timestamp));
            self.print_text(format!("Capturing screen from link: {}", url));

            tab.navigate_to(url.as_str())?;
            std::thread::sleep(Duration::from_secs(5));
            let data = tab.capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)?;
            std::fs::write(&destination, data)?;

            self.print_text(format!("Captured image and saved to: {}", destination.display()))
        }

        Ok(())
    }
}

#[derive(Clone, Copy, EnumIter, Default)]
enum Panel {
    #[default]
    FinalizationRate,
    RunningReplicas,
}

impl Panel {
    fn get_name(&self) -> &str {
        match self {
            Panel::FinalizationRate => "FinalizationRate",
            Panel::RunningReplicas => "RunningReplicas",
        }
    }

    fn get_dashboard(&self) -> &str {
        match self {
            Panel::FinalizationRate => "/d/ic-progress-clock/ic-progress-clock",
            Panel::RunningReplicas => "/d/ic-progress-clock/ic-progress-clock",
        }
    }
}

impl From<Panel> for String {
    fn from(value: Panel) -> Self {
        match value {
            Panel::FinalizationRate => "4",
            Panel::RunningReplicas => "32",
        }
        .to_string()
    }
}
