#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::process::Command;
    use assert_cmd::cargo::CommandCargoExt;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tokio::time::sleep;
    use multiservice_discovery_shared::builders::prometheus_config_structure::{PrometheusStaticConfig, JOB, IC_NAME, IC_NODE, IC_SUBNET};

    async fn fetch_targets() -> anyhow::Result<BTreeSet<PrometheusStaticConfig>> {
        let timeout_duration = Duration::from_secs(300);
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Timeout reached"));
            }
            sleep(Duration::from_secs(5)).await;

            let response = reqwest::get("http://localhost:8000/prom/targets").await?.text().await?;
            let deserialized: Result<BTreeSet<PrometheusStaticConfig>, serde_json::Error> = serde_json::from_str(&response);

            match deserialized {
                Ok(mainnet_targets) => {
                    if !mainnet_targets.is_empty() {
                        return Ok(mainnet_targets);
                    }
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Failed to deserialize: {}", err));
                }
            }
        }
    }
    #[test]
    fn prom_targets_tests() {
        let rt = Runtime::new().unwrap();
        let args = [
            "--targets-dir",
            "tests/test_data",
            "--nns-url",
            "http://donotupdate.app"
        ];
        let bazel_path = "rs/ic-observability/multiservice-discovery/multiservice-discovery";

        let mut cmd = Command::cargo_bin("multiservice-discovery").unwrap_or_else(|_| Command::new(bazel_path));

        if let Ok(mut child) = cmd.args(args).spawn() {
            let handle = rt.spawn(async {
                fetch_targets().await
            });
            let targets = rt.block_on(handle).unwrap().unwrap();
            child.kill().expect("command couldn't be killed");

            assert_eq!(targets.len(), 6);

            let labels_set = targets
                .iter()
                .cloned()
                .fold(BTreeMap::new(), |mut acc: BTreeMap<String, BTreeSet<String>>, v| {
                    for (key, value) in v.labels {
                        if let Some(grouped_set) = acc.get_mut(&key) {
                            grouped_set.insert(value);
                        } else {
                            let mut new_set = BTreeSet::new();
                            new_set.insert(value);
                            acc.insert(key,new_set);
                        }
                    }
                    acc
                });
            
            assert_eq!(
                labels_set.get(IC_NAME).unwrap().iter().collect::<Vec<_>>(),
                vec!["mercury"]
            );

            assert_eq!(
                labels_set.get(JOB).unwrap().iter().collect::<Vec<_>>(),
                vec!["node_exporter", "orchestrator", "replica"]
            );

            assert_eq!(
                labels_set.get(IC_NODE).unwrap().iter().collect::<Vec<_>>(),
                vec![
                    "4uphn-ptciz-qz3q7-sk7jj-eklt6-d4gkr-ox7dp-zzdje-cudnx-v6kwi-uqe",
                    "kxkkh-hhfjy-jn7ai-zwnp3-bcnts-npet2-m3qxb-dmz5b-4o4n3-dnb4w-eqe",
                ]            
            );

            assert_eq!(
                labels_set.get(IC_SUBNET).unwrap().iter().collect::<Vec<_>>(),
                vec![
                    "jmrpx-o3j3n-g57wn-wzq2o-cxyeg-k52f5-sqkvm-rnidf-jvbsa-oe4z2-6ae",
                    "oxqbs-zdhkh-ygwt2-lofma-yotgh-maxep-vog7o-amp42-tbwc6-6llas-tae",
                ]
            );

        } else {
            panic!("yes command didn't start");
        }
    }
}
