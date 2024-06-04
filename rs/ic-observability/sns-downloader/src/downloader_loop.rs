use crossbeam_channel::Receiver;
use ic_canisters::sns_wasm::SnsWasmCanister;
use ic_canisters::IcAgentCanisterClient;
use multiservice_discovery_shared::builders::sns_canister_config_structure::SnsCanisterConfigStructure;
use multiservice_discovery_shared::contracts::deployed_sns::Sns;
use slog::{debug, info, warn, Logger};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::CliArgs;

pub async fn run_downloader_loop(logger: Logger, cli: CliArgs, stop_signal: Receiver<()>) {
    let interval = crossbeam::channel::tick(cli.poll_interval);

    let sns_canister: SnsWasmCanister = IcAgentCanisterClient::from_anonymous(cli.nns_urls[0].clone()).unwrap().into();

    let mut current_hash: u64 = 0;

    loop {
        let tick = crossbeam::select! {
            recv(stop_signal) -> _ => {
                info!(logger, "Received shutdown signal in downloader_loop");
                return
            },
            recv(interval) -> msg => msg.expect("tick failed!")
        };

        let mut targets: Vec<Sns> = match sns_canister.list_deployed_snses().await {
            Ok(r) => r.instances.into_iter().map(|d| d.into()).collect(),
            Err(e) => {
                warn!(logger, "Received error: {:?}", e);
                continue;
            }
        };

        let mut hasher = DefaultHasher::new();

        targets.sort_by_key(|f| f.root_canister_id.clone());

        for target in &targets {
            target.hash(&mut hasher);
        }

        let hash = hasher.finish();

        if current_hash != hash {
            info!(
                logger,
                "Received new targets from {} @ interval {:?}, old hash '{}' != '{}' new hash", cli.nns_urls[0], tick, current_hash, hash
            );
            current_hash = hash;

            generate_config(&cli, targets, logger.clone());
        }
    }
}

fn generate_config(cli: &CliArgs, targets: Vec<Sns>, logger: Logger) {
    if std::fs::metadata(&cli.output_dir).is_err() {
        std::fs::create_dir_all(cli.output_dir.parent().unwrap()).unwrap();
        std::fs::File::create(&cli.output_dir).unwrap();
    }

    let config = SnsCanisterConfigStructure {
        script_path: cli.script_path.clone(),
        data_folder: cli.cursors_folder.clone(),
        restart_on_exit: cli.restart_on_exit,
        include_stderr: cli.include_stderr,
    }
    .build(targets);
    let path = cli.output_dir.join("canisters.json");
    match std::fs::write(path, config) {
        Ok(_) => {}
        Err(e) => debug!(logger, "Failed to write config to file"; "err" => format!("{}", e)),
    }
}
