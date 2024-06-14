use std::collections::{BTreeMap, BTreeSet};

use ic_types::PrincipalId;
use serde::{Deserialize, Serialize, Serializer};

use crate::{builders::ConfigBuilder, contracts::target::TargetDto};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct PrometheusStaticConfig {
    pub targets: BTreeSet<String>,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PrometheusFileSdConfig {
    configs: BTreeSet<PrometheusStaticConfig>,
    updated: bool,
}

impl Serialize for PrometheusFileSdConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.configs.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PrometheusConfigBuilder {}

pub const IC_NAME: &str = "ic";
pub const IC_NODE: &str = "ic_node";
pub const IC_SUBNET: &str = "ic_subnet";
pub const JOB: &str = "job";
pub const API_BOUNDARY_NODE: &str = "api_boundary_node";
// TODO: Re-add the labels below once we resolve the issues with the public dashboard queries
// https://dfinity.atlassian.net/browse/OB-442
// const DC: &str = "dc";
// const NODE_PROVIDER_ID: &str = "node_provider_id";
// const NODE_OPERATOR_ID: &str = "node_operator_id";

pub fn map_target_group(target_groups: Vec<TargetDto>) -> Vec<PrometheusStaticConfig> {
    target_groups
        .into_iter()
        .flat_map(|tg| {
            let mut ret = vec![];
            for job in &tg.jobs {
                ret.push(PrometheusStaticConfig {
                    targets: tg.targets.iter().map(|sa| job.url(*sa, false)).collect(),
                    labels: {
                        BTreeMap::from([
                            (IC_NAME.into(), tg.ic_name.clone()),
                            (
                                IC_NODE.into(),
                                if tg.node_id.to_string() == PrincipalId::new_anonymous().to_string() {
                                    tg.name.clone()
                                } else {
                                    tg.node_id.to_string()
                                },
                            ),
                            (JOB.into(), job.to_string()),
                        ])
                        .into_iter()
                        .chain(match tg.subnet_id {
                            Some(subnet_id) => vec![(IC_SUBNET.into(), subnet_id.to_string())],
                            None => vec![],
                        })
                        .chain(match tg.is_api_bn {
                            true => vec![(API_BOUNDARY_NODE.into(), "1".into())],
                            false => vec![],
                        })
                        .chain(tg.custom_labels.clone().into_iter())
                        .collect()
                        // TODO: Re-add the labels below once we resolve the issues with the public dashboard queries
                        // https://dfinity.atlassian.net/browse/OB-442
                        // labels.insert(DC.into(), tg.dc_id.clone());
                        // labels.insert(NODE_PROVIDER_ID.into(), tg.node_provider_id.to_string());
                        // labels.insert(NODE_OPERATOR_ID.into(), tg.operator_id.to_string());
                    },
                })
            }
            ret
        })
        .collect()
}

impl ConfigBuilder for PrometheusConfigBuilder {
    fn build(&self, target_groups: BTreeSet<TargetDto>) -> String {
        let new_configs: Vec<PrometheusStaticConfig> = map_target_group(target_groups.into_iter().collect());
        serde_json::to_string_pretty(&new_configs).unwrap()
    }
}
