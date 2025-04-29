use std::collections::HashMap;

use crate::contracts::journald_target::JournaldTarget;

use super::{
    exec_log_config_structure::{SourceStreamingWrapper, VectorExecSource},
    log_vector_config_structure::VectorRemapTransform,
    vector_config_enriched::{VectorConfigEnriched, VectorSource, VectorTransform},
};

/// Used with general service discovery
#[derive(Debug, Clone)]
pub struct ExecGeneralConfigBuilderImpl {
    pub script_path: String,
    pub cursor_folder: String,
    pub restart_on_exit: bool,
    pub include_stderr: bool,
}

impl ExecGeneralConfigBuilderImpl {
    pub fn build(&self, targets: Vec<JournaldTarget>) -> String {
        let mut config = VectorConfigEnriched::new();

        for target in targets {
            let key = format!("{}-custom-target", target.name);
            let source = VectorExecSource {
                _type: "exec".to_string(),
                command: vec![
                    self.script_path.as_str(),
                    "--url",
                    format!("http://{}/entries?follow", target.target).as_str(),
                    "--name",
                    key.as_str(),
                    "--cursor-path",
                    format!("{}/{}/checkpoint.txt", self.cursor_folder, key).as_str(),
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
                mode: "streaming".to_string(),
                streaming: SourceStreamingWrapper {
                    respawn_on_exit: self.restart_on_exit,
                },
                include_stderr: self.include_stderr,
            };

            let transform = VectorRemapTransform::from_general(target, key.clone());

            let mut source_map = HashMap::new();
            source_map.insert(key.clone(), Box::new(source) as Box<dyn VectorSource>);

            let mut transform_map = HashMap::new();
            transform_map.insert(format!("{}-transform", key), Box::new(transform) as Box<dyn VectorTransform>);

            config.add_target_group(source_map, transform_map);
        }

        serde_json::to_string_pretty(&config).unwrap()
    }
}
