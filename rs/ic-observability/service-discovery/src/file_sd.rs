//! Write json-files containing target information for file-based service
//! discovery configuration of prometheus.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use super::TargetGroup;
use crate::{job_types::JobType, service_discovery_record::ServiceDiscoveryRecord};

#[derive(Clone, Debug)]
pub struct FileSd {
    /// The base directory where the configuration files will be written to.
    base_directory: PathBuf,
    /// Mapping from job name to targets.
    last_targets: Arc<RwLock<BTreeMap<String, BTreeSet<TargetGroup>>>>,
}

impl FileSd {
    pub fn new<P: AsRef<Path>>(p: P) -> Self {
        FileSd {
            base_directory: PathBuf::from(p.as_ref()),
            last_targets: Default::default(),
        }
    }

    /// Write configuration files for the job.
    ///
    /// The assumption is that no external process manipulates or deletes the written files.
    /// FileSd will memoize the calls. Thus, calling this method twice with the
    /// same arguments will have no effect.
    pub fn write_sd_config(
        &self,
        job: JobType,
        p8s_target_groups: BTreeSet<TargetGroup>,
    ) -> std::io::Result<()> {
        let mut last_targets = self.last_targets.write().unwrap();
        let last_job_targets = last_targets.entry(job.to_string()).or_default();
        if last_job_targets == &p8s_target_groups {
            return Ok(());
        }
        let job_path = self.base_directory.join(job.to_string());
        if !job_path.is_dir() {
            std::fs::create_dir(&job_path)?;
        }
        let target_path = job_path.join("ic_service_discovery.json");

        let targets: Vec<_> = p8s_target_groups
            .clone()
            .into_iter()
            .map(ServiceDiscoveryRecord::from)
            .collect();
        ic_utils::fs::write_atomically(target_path.as_path(), |f| {
            serde_json::to_writer_pretty(f, &targets).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Serialization error: {:?}", e),
                )
            })
        })?;
        last_targets.insert(job.to_string(), p8s_target_groups);
        Ok(())
    }
}
