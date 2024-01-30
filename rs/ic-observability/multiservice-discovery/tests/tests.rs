// Note: If you need to update the assertions, please check the README.md file for guidelines.
#[cfg(test)]
mod tests {
    use assert_cmd::cargo::CommandCargoExt;
    use dirs::home_dir;
    use flate2::read::GzDecoder;
    use multiservice_discovery_shared::builders::prometheus_config_structure::{
        PrometheusStaticConfig, IC_NAME, IC_SUBNET, JOB,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Duration;
    use std::{fs, thread};
    use tar::Archive;
    use tokio::runtime::Runtime;

    const BAZEL_SD_PATH: &str = "rs/ic-observability/multiservice-discovery/multiservice-discovery";
    const CARGO_SD_PATH: &str = "multiservice-discovery";
    const BAZEL_REGISTRY_PATH: &str = "external/mainnet_registry";
    const CARGO_REGISTRY_PATH: &str = ".cache/mainnet_registry";

    const REGISTRY_MAINNET_URL: &str = "https://github.com/dfinity/dre/raw/ic-registry-mainnet/rs/ic-observability/multiservice-discovery/tests/test_data/mercury.tar.gz";

    async fn fetch_targets() -> anyhow::Result<BTreeSet<PrometheusStaticConfig>> {
        let timeout_duration = Duration::from_secs(300);
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Timeout reached"));
            }
            if let Ok(response) = reqwest::get("http://localhost:8000/prom/targets").await {
                if response.status().is_success() {
                    let text = response.text().await?;
                    let targets: BTreeSet<PrometheusStaticConfig> = serde_json::from_str(&text).unwrap();
                    return Ok(targets);
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    }

    async fn download_and_extract(url: &str, output_target_dir: &Path) -> anyhow::Result<()> {
        let response = reqwest::get(url).await?.bytes().await?;
        let decoder = GzDecoder::new(&response[..]);
        let mut archive = Archive::new(decoder);
        archive.unpack(output_target_dir)?;

        Ok(())
    }

    async fn command_from_env() -> anyhow::Result<(std::process::Command, PathBuf)> {
        if let Ok(command) = Command::cargo_bin(CARGO_SD_PATH) {
            // Runs cargo test
            let mut registry_cache_dir = home_dir().unwrap();
            registry_cache_dir.push(CARGO_REGISTRY_PATH);
            if !fs::metadata(registry_cache_dir.as_path()).is_ok() {
                download_and_extract(REGISTRY_MAINNET_URL, registry_cache_dir.as_path()).await?;
            }

            Ok((command, registry_cache_dir))
        } else {
            // Runs bazel test
            let registry_path = PathBuf::from(BAZEL_REGISTRY_PATH);
            let command = Command::new(BAZEL_SD_PATH);
            Ok((command, registry_path))
        }
    }

    #[test]
    fn prom_targets_tests() {
        let rt = Runtime::new().unwrap();
        let (mut command, registry_path) = rt
            .block_on(rt.spawn(async { command_from_env().await }))
            .unwrap()
            .unwrap();

        let args = vec![
            "--nns-url",
            "http://donotupdate.app",
            "--targets-dir",
            registry_path.as_path().to_str().unwrap(),
        ];

        let mut sd_server = command.args(args).spawn().unwrap();
        let targets = rt.block_on(rt.spawn(async { fetch_targets().await })).unwrap().unwrap();
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

        assert_eq!(targets.len(), 7359);

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

        let subnets = labels_set.get(IC_SUBNET).unwrap().iter().cloned().collect::<Vec<_>>();
        let expected_subnets = vec![
            "2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe",
            "3hhby-wmtmw-umt4t-7ieyg-bbiig-xiylg-sblrt-voxgt-bqckd-a75bf-rqe",
            "4ecnw-byqwz-dtgss-ua2mh-pfvs7-c3lct-gtf4e-hnu75-j7eek-iifqm-sqe",
            "4zbus-z2bmt-ilreg-xakz4-6tyre-hsqj4-slb4g-zjwqo-snjcc-iqphi-3qe",
            "5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae",
            "6pbhf-qzpdk-kuqbr-pklfa-5ehhf-jfjps-zsj6q-57nrl-kzhpd-mu7hc-vae",
            "bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe",
            "brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae",
            "csyj4-zmann-ys6ge-3kzi6-onexi-obayx-2fvak-zersm-euci4-6pslt-lae",
            "cv73p-6v7zi-u67oy-7jc3h-qspsz-g5lrj-4fn7k-xrax3-thek2-sl46v-jae",
            "e66qm-3cydn-nkf4i-ml4rb-4ro6o-srm5s-x5hwq-hnprz-3meqp-s7vks-5qe",
            "ejbmu-grnam-gk6ol-6irwa-htwoj-7ihfl-goimw-hlnvh-abms4-47v2e-zqe",
            "eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe",
            "fuqsr-in2lc-zbcjj-ydmcw-pzq7h-4xm2z-pto4i-dcyee-5z4rz-x63ji-nae",
            "gmq5v-hbozq-uui6y-o55wc-ihop3-562wb-3qspg-nnijg-npqp5-he3cj-3ae",
            "io67a-2jmkw-zup3h-snbwi-g6a5n-rm5dn-b6png-lvdpl-nqnto-yih6l-gqe",
            "jtdsg-3h6gi-hs7o5-z2soi-43w3z-soyl3-ajnp3-ekni5-sw553-5kw67-nqe",
            "k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe",
            "lhg73-sax6z-2zank-6oer2-575lz-zgbxx-ptudx-5korm-fy7we-kh4hl-pqe",
            "lspz2-jx4pu-k3e7p-znm7j-q4yum-ork6e-6w4q6-pijwq-znehu-4jabe-kqe",
            "mpubz-g52jc-grhjo-5oze5-qcj74-sex34-omprz-ivnsm-qvvhr-rfzpv-vae",
            "nl6hn-ja4yw-wvmpy-3z2jx-ymc34-pisx3-3cp5z-3oj4a-qzzny-jbsv3-4qe",
            "o3ow2-2ipam-6fcjo-3j5vt-fzbge-2g7my-5fz2m-p4o2t-dwlc4-gt2q7-5ae",
            "opn46-zyspe-hhmyp-4zu6u-7sbrh-dok77-m7dch-im62f-vyimr-a3n2c-4ae",
            "pae4o-o6dxf-xki7q-ezclx-znyd6-fnk6w-vkv5z-5lfwh-xym2i-otrrw-fqe",
            "pjljw-kztyl-46ud4-ofrj6-nzkhm-3n4nt-wi3jt-ypmav-ijqkt-gjf66-uae",
            "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae",
            "qdvhd-os4o2-zzrdw-xrcv4-gljou-eztdp-bj326-e6jgr-tkhuc-ql6v2-yqe",
            "qxesv-zoxpm-vc64m-zxguk-5sj74-35vrb-tbgwg-pcird-5gr26-62oxl-cae",
            "shefu-t3kr5-t5q3w-mqmdq-jabyv-vyvtf-cyyey-3kmo4-toyln-emubw-4qe",
            "snjp4-xlbw4-mnbog-ddwy6-6ckfd-2w5a2-eipqo-7l436-pxqkh-l6fuv-vae",
            "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe",
            "uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe",
            "w4asl-4nmyj-qnr7c-6cqq4-tkwmt-o26di-iupkq-vx4kt-asbrx-jzuxh-4ae",
            "w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe",
            "x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae",
            "yinp6-35cfo-wgcd2-oc4ty-2kqpf-t4dul-rfk33-fsq3r-mfmua-m2ngh-jqe",
        ];

        for subnet in expected_subnets {
            assert!(subnets.contains(&subnet.to_string()))
        }
    }
}
