use ic_types::{PrincipalId, SubnetId};
use serde::Deserialize;
use service_discovery::TargetGroup;

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttributesFilter {
    ic_name: Option<String>,
    dc_id: Option<String>,
    operator_id: Option<PrincipalId>,
    subnet_id: Option<SubnetId>,
    node_provider_id: Option<PrincipalId>
}

impl AttributesFilter {
    fn filter_step<T: PartialEq>(maybe_want: &Option<T>, got: &T) -> bool {
        maybe_want.as_ref().map_or(true, |want| want == got)
    }

    pub(crate) fn filter(&self, target_groups: &TargetGroup) -> bool {
        Self::filter_step(&self.ic_name, &target_groups.ic_name) &&
        Self::filter_step(&self.dc_id, &target_groups.dc_id) &&
        Self::filter_step(&self.operator_id, &target_groups.operator_id) &&
        Self::filter_step(&self.node_provider_id, &target_groups.node_provider_id) &&
        match (self.subnet_id, target_groups.subnet_id) {
            (Some(id_a), Some(id_b)) => id_a == id_b,
            (Some(_), _) => false,
            _ => true
        }
    }
}
