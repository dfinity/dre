use std::collections::HashMap;

use serde::Serialize;

use crate::contracts::deployed_sns::Sns;

use super::{
    log_vector_config_structure::VectorRemapTransform,
    vector_config_enriched::{VectorConfigEnriched, VectorSource, VectorTransform},
};

#[derive(Debug, Clone)]
pub struct SnsCanisterConfigStructure {
    pub script_path: String,
    pub data_folder: String,
    pub restart_on_exit: bool,
    pub include_stderr: bool,
}

// Scrapable types are: root, swap, governance
impl SnsCanisterConfigStructure {
    pub fn build(&self, snses: Vec<Sns>) -> String {
        let mut config = VectorConfigEnriched::new();

        for sns in snses {
            if sns.root_canister_id != String::default() {
                self.insert_into_config(&mut config, &sns.root_canister_id, "root", &sns.name);
            }
            if sns.swap_canister_id != String::default() {
                self.insert_into_config(&mut config, &sns.swap_canister_id, "swap", &sns.name);
            }
            if sns.governance_canister_id != String::default() {
                self.insert_into_config(&mut config, &sns.governance_canister_id, "governance", &sns.name);
            }
        }

        serde_json::to_string_pretty(&config).unwrap()
    }

    fn insert_into_config(&self, config: &mut VectorConfigEnriched, canister_id: &str, canister_type: &str, sns_name: &str) {
        let source = VectorScriptSource {
            _type: "exec".to_string(),
            command: [
                self.script_path.as_str(),
                "--url",
                format!("https://{}.raw.icp0.io/logs", canister_id).as_str(),
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            mode: "streaming".to_string(),
            streaming: SourceStreamingWrapper {
                respawn_on_exit: self.restart_on_exit,
            },
            include_stderr: self.include_stderr,
        };

        let transform = VectorRemapTransform {
            _type: "remap".to_string(),
            inputs: vec![canister_id.to_string()],
            source: vec![("canister_id", canister_id), ("canister_type", canister_type), ("sns_name", sns_name)]
                .into_iter()
                .map(|(k, v)| format!(".{} = \"{}\"", k, v))
                .collect::<Vec<String>>()
                .join("\n"),
        };

        let mut sources = HashMap::new();
        sources.insert(canister_id.to_string(), Box::new(source) as Box<dyn VectorSource>);

        let mut transforms = HashMap::new();
        transforms.insert(format!("{}-transform", canister_id), Box::new(transform) as Box<dyn VectorTransform>);

        config.add_target_group(sources, transforms)
    }
}

#[derive(Debug, Clone, Serialize)]
struct VectorScriptSource {
    #[serde(rename = "type")]
    _type: String,
    command: Vec<String>,
    mode: String,
    streaming: SourceStreamingWrapper,
    include_stderr: bool,
}

impl VectorSource for VectorScriptSource {
    fn clone_dyn(&self) -> Box<dyn VectorSource> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
struct SourceStreamingWrapper {
    respawn_on_exit: bool,
}
