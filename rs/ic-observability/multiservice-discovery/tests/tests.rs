#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use assert_cmd::cargo::CommandCargoExt;
    use multiservice_discovery_shared::builders::prometheus_config_structure::{
        PrometheusStaticConfig, IC_NAME, IC_NODE, IC_SUBNET, JOB,
    };
    use reqwest::IntoUrl;
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::Cursor;
    use std::path::Path;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    const BAZEL_SD_BIN: &str = "rs/ic-observability/multiservice-discovery/multiservice-discovery";
    const CARGO_SD_BIN: &str = "multiservice-discovery";
    const API_NODES_URL: &str = "https://ic-api.internetcomputer.org/api/v3/nodes";

    async fn reqwest_retry<T: IntoUrl + std::marker::Copy>(
        url: T,
        timeout: Duration,
    ) -> anyhow::Result<reqwest::Response> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to build reqwest client");

        let start_time = std::time::Instant::now();
        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout reached"));
            }
            if let Result::Ok(response) = client.get(url).send().await {
                if response.status().is_success() {
                    return Ok(response);
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    }

    async fn download_and_extract(url: &str, output_target_path: &Path) -> anyhow::Result<()> {
        let response = reqwest::get(url).await?.bytes().await?;
        zip_extract::extract(Cursor::new(response), output_target_path, false)?;
        Ok(())
    }

    #[derive(Debug, PartialEq)]
    pub struct TestData {
        keys: Vec<String>,
        ic_name: Vec<String>,
        jobs: Vec<String>,
        nodes: Vec<String>,
        subnets: Vec<String>,
    }
    impl TestData {
        fn from_prom(targets: Vec<PrometheusStaticConfig>) -> Self {
            let labels_set =
                targets
                    .iter()
                    .cloned()
                    .fold(BTreeMap::new(), |mut acc: BTreeMap<String, BTreeSet<String>>, v| {
                        for (key, value) in v.labels {
                            if let Some(grouped_set) = acc.get_mut(&key) {
                                grouped_set.insert(value);
                            } else {
                                let mut new_set = BTreeSet::new();
                                new_set.insert(value);
                                acc.insert(key, new_set);
                            }
                        }
                        acc
                    });

            Self {
                keys: labels_set.keys().cloned().collect::<Vec<_>>(),
                ic_name: labels_set.get(IC_NAME).unwrap().iter().cloned().collect::<Vec<_>>(),
                jobs: labels_set.get(JOB).unwrap().iter().cloned().collect::<Vec<_>>(),
                nodes: labels_set.get(IC_NODE).unwrap().iter().cloned().collect::<Vec<_>>(),
                subnets: labels_set.get(IC_SUBNET).unwrap().iter().cloned().collect::<Vec<_>>(),
            }
        }

        fn from_expected(nodes: Vec<String>, subnets: Vec<String>) -> Self {
            Self {
                nodes,
                subnets,
                keys: vec!["ic", "ic_node", "ic_subnet", "job"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                ic_name: vec!["mercury"].into_iter().map(String::from).collect(),
                jobs: vec![
                    "guest_metrics_proxy",
                    "host_metrics_proxy",
                    "host_node_exporter",
                    "node_exporter",
                    "orchestrator",
                    "replica",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            }
        }
    }

    pub struct SDRunner {
        command: Command,
    }
    impl SDRunner {
        async fn fetch_targets(&mut self) -> anyhow::Result<Vec<PrometheusStaticConfig>> {
            let registry_dir = tempdir().unwrap();
            const TARGETS_URL: &str = "http://localhost:8000/prom/targets";
            const REQWEST_TIMEOUT: Duration = Duration::from_secs(240);

            let args = vec!["--targets-dir", registry_dir.path().to_str().unwrap()];
            let mut sd_server = self.command.args(args).spawn().unwrap();
            let targets: Vec<PrometheusStaticConfig> =
                reqwest_retry(TARGETS_URL, REQWEST_TIMEOUT).await?.json().await?;
            sd_server.kill().unwrap();
            return Ok(targets);
        }

        pub fn from_local_bin() -> Self {
            Self {
                command: Command::cargo_bin(CARGO_SD_BIN).unwrap_or(Command::new(BAZEL_SD_BIN)),
            }
        }

        pub async fn from_remote_bin(sd_url: &str) -> Self {
            let sd_dir = tempdir().unwrap();
            let sd_bin_path = sd_dir.path().join("multiservice-discovery");
            download_and_extract(sd_url, sd_bin_path.as_path()).await.unwrap();
            Self {
                command: Command::new(sd_bin_path),
            }
        }
    }

    pub struct ExpectedDataFetcher;
    impl ExpectedDataFetcher {
        async fn from_public_dashboard_api(&self) -> anyhow::Result<TestData> {
            const REQWEST_TIMEOUT: Duration = Duration::from_secs(15);
            let response: Value = reqwest_retry(API_NODES_URL, REQWEST_TIMEOUT).await?.json().await?;
            let mut subnets = BTreeSet::new();

            let nodes = response["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|val| {
                    if let Some(sub) = val["subnet_id"].as_str() {
                        subnets.insert(String::from(sub));
                    }

                    String::from(val["node_id"].as_str().unwrap())
                })
                .collect::<Vec<_>>();

            Ok(TestData::from_expected(
                nodes,
                subnets.iter().cloned().collect::<Vec<_>>(),
            ))
        }

        async fn from_main_sd(&self) -> anyhow::Result<TestData> {
            const MAIN_SD_URL: &str = "";
            let targets: Vec<PrometheusStaticConfig> =
                SDRunner::from_remote_bin(MAIN_SD_URL).await.fetch_targets().await?;

            Ok(TestData::from_prom(targets))
        }

        pub async fn get_expected_data(&self) -> anyhow::Result<TestData> {
            // TODO: Add support for getting TestData from main MSD bin
            self.from_public_dashboard_api()
                .await
                .or(Err(anyhow!("Expected data not found")))
        }
    }

    #[tokio::test]
    async fn prom_targets_tests() {
        let expected_data = ExpectedDataFetcher.get_expected_data().await.unwrap();
        let targets: Vec<PrometheusStaticConfig> = SDRunner::from_local_bin().fetch_targets().await.unwrap();

        let test_data = TestData::from_prom(targets);

        assert_eq!(test_data, expected_data);
    }
}
