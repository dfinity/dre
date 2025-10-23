use std::collections::BTreeSet;
use std::fmt::Debug;

use service_discovery::{TargetGroup, job_types::JobType};

pub trait Config: erased_serde::Serialize + Debug {
    fn updated(&self) -> bool;
    fn name(&self) -> String;
}
erased_serde::serialize_trait_object!(Config);

pub trait ConfigBuilder {
    fn build(&mut self, target_groups: BTreeSet<TargetGroup>, job_type: JobType) -> Box<dyn Config>;
}
