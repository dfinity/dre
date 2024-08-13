use std::net::Ipv6Addr;

use chrono::Utc;
use comfy_table::CellAlignment;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use reqwest::ClientBuilder;
use serde_json::Value;
use tokio::process::Command;

use super::{
    comfy_table_util::Table,
    step::Step,
    util::{StepCtx, REQWEST_TIMEOUT},
};

const IC_WORKLOAD_GENERATOR: &str = "ic-workload-generator";

const RUNTIME_SECS: u8 = 120;
const TIMEOUT: u8 = 59;

pub struct Workload {
    pub version: String,
    pub deployment_name: String,
    pub prometheus_endpoint: String,
}

impl Step for Workload {
    fn help(&self) -> String {
        format!("Run workload test on version {} for network {}", self.version, self.deployment_name)
    }

    fn name(&self) -> String {
        "workload_test".to_string()
    }

    async fn execute(&self, ctx: &StepCtx) -> anyhow::Result<()> {
        let wg_binary = ctx.download_executable(IC_WORKLOAD_GENERATOR, &self.version).await?;

        let subnets = ctx.dre_ctx().registry().await.subnets().await?;
        let subnet = subnets
            .values()
            .find(|s| s.subnet_type.eq(&SubnetType::Application))
            .ok_or(anyhow::anyhow!("Application subnet required for step `{}`", self.name()))?;

        let all_ipv6 = subnet.nodes.iter().map(|n| n.ip_addr).collect_vec();
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

        ctx.print_text(format!("Spawning the command: {} {}", wg_binary.display(), args.iter().join(" ")));

        // Possible `ulimit` issue
        let start = Utc::now();
        let status = Command::new(wg_binary).args(args).status().await?;
        let end = Utc::now();
        let elapsed = end.signed_duration_since(start);

        if !status.success() {
            anyhow::bail!("Failed to run workload test with status code: {}", status.code().unwrap_or_default())
        }

        // No need to stop the qualification if taking picture fails
        if let Err(e) = ctx.capture_progress_clock(
            self.deployment_name.to_string(),
            &subnet.principal,
            Some(start.timestamp()),
            Some(end.timestamp()),
            "workload_test",
        ) {
            ctx.print_text(format!("Failed to capture progress clock: {:?}", e))
        };

        match ensure_finalization_rate_for_subnet(
            &self.deployment_name,
            end.timestamp(),
            elapsed.num_seconds(),
            &all_ipv6,
            &self.prometheus_endpoint,
            &SubnetType::Application,
            ctx,
        )
        .await?
        {
            true => Ok(()),
            false => Err(anyhow::anyhow!("Finalization dropped after the test")),
        }
    }
}

const REPLICA_JOB: &str = "replica";
async fn ensure_finalization_rate_for_subnet(
    deployment_name: &str,
    end_timestamp: i64,
    duration: i64,
    ips: &[Ipv6Addr],
    prom_endpoint: &str,
    subnet_type: &SubnetType,
    ctx: &StepCtx,
) -> anyhow::Result<bool> {
    let client = ClientBuilder::new().timeout(REQWEST_TIMEOUT).build()?;
    let metrics_hosts = ips.iter().map(|ip| format!("\\\\[{}\\\\]:9090", ip)).join("|");

    let common_labels = format!("ic=\"{}\",job=\"{}\",instance=~\"{}\"", deployment_name, REPLICA_JOB, metrics_hosts);
    let query_selector = format!(
        "artifact_pool_consensus_height_stat{{{},type=\"finalization\",pool_type=\"validated\",stat=\"max\"}}",
        common_labels
    );
    let query = format!("avg(rate({}[{}s]))", query_selector, duration);
    let request = client
        .get(prom_endpoint)
        .header("Accept", "application/json")
        .query(&[("time", end_timestamp.to_string()), ("query", query)]);
    ctx.print_text(format!("Running query: {:?}", request));
    let response = request.send().await?.error_for_status()?.json::<Value>().await?;
    ctx.print_text(format!("Received response: \n{}", serde_json::to_string_pretty(&response)?));

    let finalization_rate = response["data"]["result"][0]["value"][1]
        .as_str()
        .ok_or(anyhow::anyhow!("Response is not in the expected format {}", response.to_string()))?
        .parse::<f64>()?;

    let expected_finalization_rate = expected_finalization_rate_for_subnet(subnet_type, ips.len());
    let table = Table::new()
        .with_columns(&[("Expected", CellAlignment::Center), ("Achieved", CellAlignment::Center)])
        .with_rows(vec![vec![expected_finalization_rate.to_string(), finalization_rate.to_string()]])
        .to_table();

    ctx.print_table(table);

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
