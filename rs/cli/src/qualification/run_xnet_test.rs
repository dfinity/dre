use std::{
    io::{Read, Write},
    net::Ipv6Addr,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::Duration,
};

use chrono::Utc;
use flate2::read::GzDecoder;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use serde_json::Value;
use tokio::process::Command;

use crate::ctx::DreContext;

use super::{
    print_table, print_text,
    tabular_util::{ColumnAlignment, Table},
    Step,
};

const IC_WORKLOAD_GENERATOR: &str = "ic-workload-generator";
const E2E_TEST_DRIVER: &str = "e2e-test-driver";
const XNET_TEST_CANISTER: &str = "xnet-test-canister";

const IC_EXECUTABLES_DIR: &str = "ic-executables";
const RUNTIME_SECS: u8 = 120;
const TIMEOUT: u8 = 59;

pub struct XNetTest {
    pub version: String,
    pub deployment_name: String,
    pub prometheus_endpoint: String,
}

impl Step for XNetTest {
    fn help(&self) -> String {
        "This step runs the workload test on one app subnet.".to_string()
    }

    fn name(&self) -> String {
        "xnet_test".to_string()
    }

    async fn execute(&self, ctx: &DreContext) -> anyhow::Result<()> {
        let client = ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;
        for executable in &[IC_WORKLOAD_GENERATOR, E2E_TEST_DRIVER] {
            let exe_path = construct_executable_path(executable, &self.version)?;

            if exe_path.exists() && exe_path.is_file() {
                let permissions = exe_path.metadata()?.permissions();
                let is_executable = permissions.mode() & 0o111 != 0;
                if is_executable {
                    print_text(format!("Executable `{}` already present and executable", executable));
                    continue;
                }
            }

            let url = format!(
                "https://download.dfinity.systems/ic/{}/binaries/x86_64-{}/{}.gz",
                &self.version,
                match std::env::consts::OS {
                    "linux" => "x86_64-unknown-linux",
                    "macos" => "x86_64-apple-darwin",
                    s => return Err(anyhow::anyhow!("Unsupported os: {}", s)),
                },
                executable
            );

            print_text(format!("Downloading: {}", url));
            let response = client.get(&url).send().await?.error_for_status()?.bytes().await?;
            let mut d = GzDecoder::new(&response[..]);
            let mut collector: Vec<u8> = vec![];
            let mut file = std::fs::File::create(&exe_path)?;
            d.read(&mut collector)?;

            file.write_all(&collector)?;
            print_text(format!("Downloaded: {}", &url));

            file.set_permissions(PermissionsExt::from_mode(0o774))?;
            print_text(format!("Created executable: {}", exe_path.display()))
        }

        for canister in &[XNET_TEST_CANISTER] {
            let canister_path = construct_executable_path(canister, &self.version)?;

            if canister_path.exists() {
                print_text(format!("Canister `{}` data already present", canister));
                continue;
            }

            let url = format!("https://download.dfinity.systems/ic/{}/canisters/{}.wasm.gz", &self.version, canister);

            print_text(format!("Downloading: {}", url));
            let response = client.get(&url).send().await?.error_for_status()?.bytes().await?;
            let mut file = std::fs::File::create(canister_path)?;

            file.write_all(&response[..])?;
            print_text(format!("Downloaded: {}", url));
        }

        let subnets = ctx.registry().await.subnets().await?;
        let subnet = subnets
            .values()
            .find(|s| s.subnet_type.eq(&SubnetType::Application))
            .ok_or(anyhow::anyhow!("Application subnet required for step `{}`", self.name()))?;

        let all_ipv6 = subnet.nodes.iter().map(|n| n.ip_addr).collect_vec();
        let wg_binary = construct_executable_path(IC_WORKLOAD_GENERATOR, &self.version)?;
        let args = &[
            all_ipv6.iter().map(|ip| format!("http://[{}]:8080/", ip)).join(","),
            "-m=UpdateCounter".to_string(),
            "-r=100".to_string(),
            "--payload-size=1k".to_string(),
            format!("-n={}", RUNTIME_SECS),
            "--periodic-output".to_string(),
            format!("--query-timeout-secs={}", TIMEOUT),
            format!("--ingress-timeout-secs={}", TIMEOUT),
        ];

        print_text(format!("Spawning the command: {} {}", wg_binary.display(), args.iter().join(" ")));

        // Possible `ulimit` issue
        let start = Utc::now();
        let status = Command::new(wg_binary).args(args).status().await?;
        let end = Utc::now();
        let elapsed = end.signed_duration_since(start);

        if !status.success() {
            anyhow::bail!("Failed to run xnet test with status code: {}", status.code().unwrap_or_default())
        }

        match ensure_finalization_rate_for_subnet(
            &self.deployment_name,
            end.timestamp_millis(),
            elapsed.num_milliseconds(),
            &all_ipv6,
            &client,
            &self.prometheus_endpoint,
            &SubnetType::Application,
        )
        .await?
        {
            true => Ok(()),
            false => Err(anyhow::anyhow!("Finalization dropped after the test")),
        }
    }
}

fn construct_executable_path(artifact: &str, version: &str) -> anyhow::Result<PathBuf> {
    let cache = dirs::cache_dir().ok_or(anyhow::anyhow!("Can't cache dir"))?.join(IC_EXECUTABLES_DIR);
    if !cache.exists() {
        std::fs::create_dir_all(&cache)?;
    }

    let artifact_path = cache.join(format!("{}/{}.{}", artifact, artifact, version));
    let artifact_dir = artifact_path.parent().unwrap();
    if !artifact_dir.exists() {
        std::fs::create_dir(artifact_dir)?;
    }

    Ok(artifact_path)
}

const REPLICA_JOB: &str = "replica";
async fn ensure_finalization_rate_for_subnet(
    deployment_name: &str,
    end_timestamp: i64,
    duration: i64,
    ips: &[Ipv6Addr],
    client: &Client,
    prom_endpoint: &str,
    subnet_type: &SubnetType,
) -> anyhow::Result<bool> {
    let metrics_hosts = ips.iter().map(|ip| format!("\\\\[{}\\\\]:9090", ip)).join("|");

    let common_labels = format!("ic=\"{}\",job=\"{}\",instance=~\"{}\"", deployment_name, REPLICA_JOB, metrics_hosts);
    let query_selector = format!(
        "artifact_pool_consensus_height_stat{{{},type=\"finalization\",pool_type=\"validated\",stat=\"max\"}}",
        common_labels
    );

    let response = client
        .get(prom_endpoint)
        .header("Accept", "application/json")
        .query(&[
            ("time", end_timestamp.to_string()),
            ("query", format!("avg(rate({}[{}]))", query_selector, duration)),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let finalization_rate = response["data"]["result"][0]["value"][1]
        .as_f64()
        .ok_or(anyhow::anyhow!("Response is not in the expected format {}", response.to_string()))?;

    let expected_finalization_rate = expected_finalization_rate_for_subnet(subnet_type, ips.len());
    let table = Table::new()
        .with_columns(&[("Expected", ColumnAlignment::Middle), ("Achieved", ColumnAlignment::Middle)])
        .with_rows(vec![vec![expected_finalization_rate.to_string(), finalization_rate.to_string()]])
        .to_table();

    print_table(table);

    // Capture the image of grafana links
    // logging.info("Check the Grafana dashboard (adjust the subnets if necessary)"))
    //    logging.info(
    //        "Grafana URL: https://grafana.testnet.dfinity.network/d/ic-progress-clock/ic-progress-clock?orgId=1&var-ic=%s&refresh=30s",
    //        deployment_name,
    //    )
    //    logging.info(
    //        "Grafana URL: https://grafana.testnet.dfinity.network/d/execution-metrics/execution-metrics?orgId=1&var-ic=%s",
    //        deployment_name,
    //    )

    Ok(finalization_rate >= expected_finalization_rate)
}

const XL_SUBNET_SIZE: usize = 55;
fn expected_finalization_rate_for_subnet(subnet_type: &SubnetType, subnet_size: usize) -> f64 {
    if subnet_size > XL_SUBNET_SIZE {
        return 0.24;
    } else if subnet_type.eq(&SubnetType::System) {
        return 0.3;
    }
    0.9
}
