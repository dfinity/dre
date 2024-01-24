#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::process::Command;
    use assert_cmd::cargo::CommandCargoExt;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tokio::time::sleep;
    use multiservice_discovery_shared::builders::prometheus_config_structure::{PrometheusStaticConfig, JOB, IC_NAME, IC_SUBNET};

    const CRAGO_BIN_PATH: &str = "multiservice-discovery";
    const CRAGO_DATA_PATH: &str = "tests/test_data";
    const BAZEL_BIN_PATH: &str = "rs/ic-observability/multiservice-discovery/multiservice-discovery";
    const BAZEL_DATA_PATH: &str = "rs/ic-observability/multiservice-discovery/tests/test_data";

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
        let mut args = vec![
            "--nns-url",
            "http://donotupdate.app",
            "--targets-dir",
        ];
        let mut cmd = match Command::cargo_bin(CRAGO_BIN_PATH) {
            Ok(command) => {
                args.push(CRAGO_DATA_PATH);
                command
            },
            _ => {
                args.push(BAZEL_DATA_PATH);
                Command::new(BAZEL_BIN_PATH)
            }
        };
        
        let mut child = cmd.args(args).spawn().unwrap();
        let handle = rt.spawn(async {
            fetch_targets().await
        });
        let targets = rt.block_on(handle).unwrap().unwrap();
        child.kill().expect("command couldn't be killed");

        assert_eq!(targets.len(), 72);

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
            labels_set.get(IC_SUBNET).unwrap().iter().collect::<Vec<_>>(),
            vec![
                "5mlaw-duyx2-sq67s-7czsg-3dslt-5ywba-4t346-kymcl-wpqzo-vt3zg-oqe",
                "grlkk-en3y6-lbkcb-qi4n2-hbikt-u7lhr-zwiqs-xkjt6-5mrto-3a2w7-7ae",
                "hyrk7-bxql5-toski-dz327-pajhm-ml6h6-n2fps-oxbsi-4dz2n-4ba4t-nqe",
                "quqe7-f73mp-keuge-3ywt4-k254o-kqzs2-z5gjb-2ehox-6iby5-54hbp-6qe",
                "sunxw-go5eq-un4wt-qlueh-qrde6-fq6l5-svdzo-l4g43-yopxy-rtxji-dqe"
            ]
        );
    }
}
