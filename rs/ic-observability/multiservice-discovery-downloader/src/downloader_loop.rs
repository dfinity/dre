use crossbeam_channel::Receiver;
use multiservice_discovery_shared::builders::exec_log_config_structure::ExecLogConfigBuilderImpl;
use multiservice_discovery_shared::builders::general_exec::ExecGeneralConfigBuilderImpl;
use multiservice_discovery_shared::builders::script_log_config_structure::ScriptLogConfigBuilderImpl;
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;
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
use std::path::PathBuf;
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
    if fs_err::metadata(&cli.output_dir).is_err() {
        fs_err::create_dir_all(cli.output_dir.parent().unwrap()).unwrap();
        fs_err::File::create(&cli.output_dir).unwrap();
    }

    match &cli.generator {
        crate::Generator::Log(subtype) => {
            let jobs = JobType::all_for_logs();

            let builder = match &subtype.subcommands {
                Subtype::SystemdJournalGatewayd { batch_size } => {
                    Box::new(VectorConfigBuilderImpl::new(*batch_size, subtype.port, subtype.bn_port)) as Box<dyn ConfigBuilder>
                }
                Subtype::ExecAndJournald {
                    script_path,
                    journals_folder,
                    worker_cursor_folder,
                    data_folder,
                    restart_on_exit,
                } => Box::new(ScriptLogConfigBuilderImpl {
                    script_path: script_path.clone(),
                    journals_folder: journals_folder.to_string(),
                    worker_cursor_folder: worker_cursor_folder.clone(),
                    data_folder: data_folder.clone(),
                    port: subtype.port,
                    bn_port: subtype.bn_port,
                    restart_on_exit: *restart_on_exit,
                }) as Box<dyn ConfigBuilder>,
                Subtype::Exec {
                    script_path,
                    cursors_folder: cursor_folder,
                    restart_on_exit,
                    include_stderr,
                } => Box::new(ExecLogConfigBuilderImpl {
                    bn_port: subtype.bn_port,
                    port: subtype.port,
                    script_path: script_path.clone(),
                    cursor_folder: cursor_folder.clone(),
                    restart_on_exit: *restart_on_exit,
                    include_stderr: *include_stderr,
                }) as Box<dyn ConfigBuilder>,
                // Used for general service discovery which doesn't have the same api
                Subtype::ExecGeneral {
                    script_path,
                    cursors_folder,
                    restart_on_exit,
                    include_stderr,
                } => {
                    let builder = ExecGeneralConfigBuilderImpl {
                        script_path: script_path.clone(),
                        cursor_folder: cursors_folder.clone(),
                        restart_on_exit: *restart_on_exit,
                        include_stderr: *include_stderr,
                    };

                    let targets = convert_and_filter_general_targets(targets, filters);
                    let config = builder.build(targets);

                    write_config(cli.output_dir.join("targets.json"), config, &logger);
                    return;
                }
            };
            generate_config_inner(
                jobs,
                convert_and_filter_target_dtos(targets, filters),
                logger,
                cli.output_dir.clone(),
                builder,
            );
        }
        crate::Generator::Metric => {
            let jobs = JobType::all_for_ic_nodes();
            generate_config_inner(
                jobs,
                convert_and_filter_target_dtos(targets, filters),
                logger,
                cli.output_dir.clone(),
                Box::new(PrometheusConfigBuilder {}) as Box<dyn ConfigBuilder>,
            );
        }
    }
}

fn convert_and_filter_general_targets(values: Vec<Value>, filters: &TargetGroupFilterList) -> Vec<JournaldTarget> {
    values
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .filter(|target| filters.filter(target))
        .collect()
}

fn convert_and_filter_target_dtos(values: Vec<Value>, filters: &TargetGroupFilterList) -> Vec<TargetDto> {
    values
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .filter(|target| filters.filter(target))
        .collect()
}

fn generate_config_inner(jobs: Vec<JobType>, targets: Vec<TargetDto>, logger: Logger, output_dir: PathBuf, builder: Box<dyn ConfigBuilder>) {
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

        let config = builder.build(targets_with_job);

        let path = output_dir.join(format!("{}.json", job));

        write_config(path, config, &logger);
    }
}

fn write_config(path: PathBuf, config: String, logger: &Logger) {
    match fs_err::write(&path, config) {
        Ok(_) => {}
        Err(e) => debug!(logger, "Failed to write config to file"; "err" => format!("{}", e)),
    }
}
