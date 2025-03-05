use crossbeam_channel::Receiver;
use multiservice_discovery_shared::builders::exec_log_config_structure::ExecLogConfigBuilderImpl;
use multiservice_discovery_shared::builders::script_log_config_structure::ScriptLogConfigBuilderImpl;
use multiservice_discovery_shared::filters::ic_name_regex_filter::IcNameRegexFilter;
use multiservice_discovery_shared::filters::node_regex_id_filter::NodeIDRegexFilter;
use multiservice_discovery_shared::filters::{TargetGroupFilter, TargetGroupFilterList};
use multiservice_discovery_shared::{
    builders::{log_vector_config_structure::VectorConfigBuilderImpl, prometheus_config_structure::PrometheusConfigBuilder, ConfigBuilder},
    contracts::target::TargetDto,
};
use serde_json::Value;
use service_discovery::job_types::JobType;
use slog::{debug, info, warn, Logger};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::log_subtype::Subtype;
use crate::CliArgs;

pub async fn run_downloader_loop(logger: Logger, cli: CliArgs, stop_signal: Receiver<()>) {
    let interval = crossbeam::channel::tick(cli.poll_interval);

    let client = reqwest::Client::builder()
        .timeout(cli.registry_query_timeout)
        .build()
        .expect("Failed to build reqwest client");

    let mut filters = TargetGroupFilterList::new(vec![]);

    if let Some(regex) = &cli.filter_node_id_regex {
        filters.add(Box::new(NodeIDRegexFilter::new(regex.clone())))
    }

    if let Some(regex) = &cli.filter_ic_name_regex {
        filters.add(Box::new(IcNameRegexFilter::new(regex.clone())));
    }

    let mut current_hash: u64 = 0;

    loop {
        let tick = crossbeam::select! {
            recv(stop_signal) -> _ => {
                info!(logger, "Received shutdown signal in downloader_loop");
                return
            },
            recv(interval) -> msg => msg.expect("tick failed!")
        };
        info!(logger, "Downloading from {} @ interval {:?}", cli.sd_url, tick);

        let response = match client.get(cli.sd_url.clone()).send().await {
            Ok(res) => res,
            Err(e) => {
                warn!(logger, "Failed to download from {} @ interval {:?}: {:?}", cli.sd_url, tick, e);
                continue;
            }
        };

        if !response.status().is_success() {
            warn!(logger, "Received failed status {} @ interval {:?}: {:?}", cli.sd_url, tick, response);
            continue;
        }

        let targets: Value = match response.json().await {
            Ok(targets) => targets,
            Err(e) => {
                warn!(logger, "Failed to parse response from {} @ interval {:?}: {:?}", cli.sd_url, tick, e);
                continue;
            }
        };

        let targets = match targets {
            Value::Array(array) => array,
            v => {
                warn!(logger, "Got unexpected data contract: {:?}", v);
                continue;
            }
        };

        if targets.is_empty() {
            warn!(logger, "Got zero targets, skipping @ interval {:?}", tick);
            continue;
        }

        let mut hasher = DefaultHasher::new();

        for target in &targets {
            target.hash(&mut hasher);
        }

        let hash = hasher.finish();

        if current_hash != hash {
            info!(logger, "Received new targets from {} @ interval {:?}", cli.sd_url, tick);
            current_hash = hash;

            generate_config(&cli, targets, logger.clone(), &filters);
        }
    }
}

fn generate_config(cli: &CliArgs, targets: Vec<Value>, logger: Logger, filters: &TargetGroupFilterList) {
    if matches!(
        cli.generator.clone(),
        crate::Generator::Log(subtype) if matches!(subtype.subcommands.clone(), Subtype::ExecGeneral { script_path: _, cursors_folder: _, restart_on_exit: _, include_stderr: _ })
    ) {
        println!("Will generate new config")
    } else {
        generate_config_inner(cli, convert_and_filter_target_dtos(targets, filters), logger);
    }
}

fn convert_and_filter_target_dtos(values: Vec<Value>, filters: &TargetGroupFilterList) -> Vec<TargetDto> {
    values
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .filter(|target| filters.filter(target))
        .collect()
}

fn generate_config_inner(cli: &CliArgs, targets: Vec<TargetDto>, logger: Logger) {
    let jobs = match cli.generator {
        crate::Generator::Log(_) => JobType::all_for_logs(),
        crate::Generator::Metric => JobType::all_for_ic_nodes(),
    };

    if fs_err::metadata(&cli.output_dir).is_err() {
        fs_err::create_dir_all(cli.output_dir.parent().unwrap()).unwrap();
        fs_err::File::create(&cli.output_dir).unwrap();
    }

    for job in &jobs {
        let targets_with_job = targets
            .clone()
            .iter_mut()
            .filter(|f| f.jobs.contains(job))
            .map(|f| TargetDto {
                jobs: vec![*job],
                ..f.clone()
            })
            .collect();

        let config = match &cli.generator {
            crate::Generator::Log(subtype) => match &subtype.subcommands {
                Subtype::SystemdJournalGatewayd { batch_size } => {
                    VectorConfigBuilderImpl::new(*batch_size, subtype.port, subtype.bn_port).build(targets_with_job)
                }
                Subtype::ExecAndJournald {
                    script_path,
                    journals_folder,
                    worker_cursor_folder,
                    data_folder,
                    restart_on_exit,
                } => ScriptLogConfigBuilderImpl {
                    script_path: script_path.to_string(),
                    journals_folder: journals_folder.to_string(),
                    worker_cursor_folder: worker_cursor_folder.to_string(),
                    data_folder: data_folder.to_string(),
                    port: subtype.port,
                    bn_port: subtype.bn_port,
                    restart_on_exit: *restart_on_exit,
                }
                .build(targets_with_job),
                Subtype::Exec {
                    script_path,
                    cursors_folder,
                    restart_on_exit,
                    include_stderr,
                } => ExecLogConfigBuilderImpl {
                    bn_port: subtype.bn_port,
                    port: subtype.port,
                    script_path: script_path.to_string(),
                    cursor_folder: cursors_folder.to_string(),
                    restart_on_exit: *restart_on_exit,
                    include_stderr: *include_stderr,
                }
                .build(targets_with_job),
                Subtype::ExecGeneral {
                    script_path: _,
                    cursors_folder: _,
                    restart_on_exit: _,
                    include_stderr: _,
                } => unreachable!("Shouldn't happen"),
            },
            crate::Generator::Metric => PrometheusConfigBuilder {}.build(targets_with_job),
        };

        let path = cli.output_dir.join(format!("{}.json", job));

        match fs_err::write(&path, config) {
            Ok(_) => {}
            Err(e) => debug!(logger, "Failed to write config to file"; "err" => format!("{}", e)),
        }
    }
}
