use std::{os::unix::fs::PermissionsExt, time::Duration};

use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use tokio::process::Command;

use super::{download_canister, download_executable, print_text, Step};

const E2E_TEST_DRIVER: &str = "e2e-test-driver";
const XNET_TEST_CANISTER: &str = "xnet-test-canister";
const XNET_PRINCIPAL_PATH: &str = ".config/dfx/identity/xnet-testing/identity.pem";

const NUM_SUBNETS: usize = 2;
const RUNTIME: Duration = Duration::from_secs(60);
const REQUEST_RATE: usize = 10;
const PAYLOAD_SIZE: usize = 1024;
const CYCLES_PER_SUBNET: u128 = 10000000000000;
const XNET_TEST_NUMBER: &str = "4.3";

pub struct RunXnetTest {
    pub version: String,
}

impl Step for RunXnetTest {
    fn help(&self) -> String {
        format!("Run xnet test for version {}", self.version)
    }

    fn name(&self) -> String {
        "xnet_test".to_string()
    }

    async fn execute(&self, ctx: &crate::ctx::DreContext) -> anyhow::Result<()> {
        let key = dirs::home_dir()
            .ok_or(anyhow::anyhow!("Cannot get home directory"))?
            .join(XNET_PRINCIPAL_PATH);

        if !key.exists() {
            anyhow::bail!("Principal key for xnet testing not found at {}", key.display());
        }
        let file = std::fs::File::open(&key)?;
        file.set_permissions(PermissionsExt::from_mode(0o400))?;

        let e2e_bin = download_executable(E2E_TEST_DRIVER, &self.version).await?;
        let wasm_path = download_canister(XNET_TEST_CANISTER, &self.version).await?;

        let registry = ctx.registry().await;
        let subnet = registry.subnets().await?;
        let subnet = subnet
            .values()
            .find(|s| s.subnet_type.eq(&SubnetType::System))
            .ok_or(anyhow::anyhow!("Failed to find system subnet on the network"))?;
        let nns_node = subnet.nodes.first().ok_or(anyhow::anyhow!("Failed to find a node in a system subnet"))?;

        let args = &[
            "--nns_url".to_string(),
            format!("http://[{}]:8080/", nns_node.ip_addr),
            "--subnets".to_string(),
            NUM_SUBNETS.to_string(),
            "--principal_key".to_string(),
            key.display().to_string(),
            "--runtime".to_string(),
            RUNTIME.as_secs().to_string(),
            "--rate".to_string(),
            REQUEST_RATE.to_string(),
            "--payload_size".to_string(),
            PAYLOAD_SIZE.to_string(),
            "--cycles_per_subnet".to_string(),
            CYCLES_PER_SUBNET.to_string(),
            "--".to_string(),
            XNET_TEST_NUMBER.to_string(),
        ];

        print_text(format!(
            "Running command: XNET_TEST_CANISTER_WASM_PATH={} {} {}",
            wasm_path.display(),
            e2e_bin.display(),
            args.iter().join(" ")
        ));

        let status = Command::new(e2e_bin)
            .args(args)
            .env("XNET_TEST_CANISTER_WASM_PATH", wasm_path.display().to_string())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to run xnet test with status code: {}", status.code().unwrap_or_default())
        }

        Ok(())
    }
}
