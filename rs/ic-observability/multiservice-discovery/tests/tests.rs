// Note: If you need to update the assertions, please check the README.md file for guidelines.
#[cfg(test)]
mod tests {
    use assert_cmd::cargo::CommandCargoExt;
    use multiservice_discovery_shared::builders::prometheus_config_structure::{
        PrometheusStaticConfig, IC_NAME, IC_NODE, IC_SUBNET, JOB
    };
    use serde_json::Value;
    use tempfile::tempdir;
    use std::collections::{BTreeMap, BTreeSet};
    use std::process::Command;
    use std::time::Duration;
    use std::thread;
    use tokio::runtime::Runtime;

    const BAZEL_SD_BIN: &str = "rs/ic-observability/multiservice-discovery/multiservice-discovery";
    const CARGO_SD_BIN: &str = "multiservice-discovery";

    async fn fetch_targets() -> anyhow::Result<Vec<PrometheusStaticConfig>> {
        const TARGETS_URL: &str = "http://localhost:8000/prom/targets";
        let timeout_duration = Duration::from_secs(500);
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Timeout reached"));
            }
            if let Ok(response) = reqwest::get(TARGETS_URL).await {
                if response.status().is_success() {
                    let text = response.text().await?;
                    let targets: Vec<PrometheusStaticConfig> = serde_json::from_str(&text).unwrap();
                    return Ok(targets);
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    }


    async fn expected_nodes_and_subnets() -> Result<(Vec<String>, Vec<String>), reqwest::Error> {
        const API_NODES_URL: &str = "https://ic-api.internetcomputer.org/api/v3/nodes";
        let response = reqwest::get(API_NODES_URL).await?.text().await?;
        let response_value: Value = serde_json::from_str(&response).unwrap();
        let mut subnets = BTreeSet::new();
        
        let nodes = response_value["nodes"].as_array().unwrap()
            .iter()
            .map(|val| {
                if let Some(sub) = val["subnet_id"].as_str() {
                    subnets.insert(String::from(sub));
                }

                String::from(val["node_id"].as_str().unwrap())
            }).collect::<Vec<_>>();
            
        Ok((nodes, subnets.into_iter().collect()))
    }

    #[test]
    fn prom_targets_tests() {
        let rt = Runtime::new().unwrap();

        let (expected_nodes, expected_subnets) = rt.block_on(rt.spawn(async { 
            expected_nodes_and_subnets().await 
        })).unwrap().unwrap();

        let registry_dir = tempdir().unwrap();
        registry_dir.path().to_str().unwrap();
        let args = vec![
            "--targets-dir",
            registry_dir.path().to_str().unwrap(),
        ];
        let mut sd_server = Command::cargo_bin(CARGO_SD_BIN)
            .unwrap_or(Command::new(BAZEL_SD_BIN))
            .args(args)
            .spawn()
            .unwrap();
        let targets = rt.block_on(rt.spawn(async { 
            fetch_targets().await 
        })).unwrap().unwrap();
        sd_server.kill().unwrap();

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
        
        let subnets = labels_set.get(IC_SUBNET).unwrap()
            .iter().cloned().collect::<Vec<_>>();
        let nodes = labels_set.get(IC_NODE).unwrap()
            .iter().cloned().collect::<Vec<_>>();

        assert_eq!(
            labels_set.keys().collect::<Vec<_>>(),
            vec!["ic", "ic_node", "ic_subnet", "job"]
        );

        assert_eq!(
            labels_set.get(IC_NAME).unwrap().iter().collect::<Vec<_>>(),
            vec!["mercury"]
        );

        assert_eq!(
            labels_set.get(JOB).unwrap().iter().collect::<Vec<_>>(),
            vec![
                "guest_metrics_proxy",
                "host_metrics_proxy",
                "host_node_exporter",
                "node_exporter",
                "orchestrator",
                "replica"
            ]
        );

        assert_eq!(subnets, expected_subnets);

        assert_eq!(nodes, expected_nodes);
    }
}
