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

impl Default for AttributesFilter {
    fn default() -> Self {
        Self { 
            ic_name: Default::default(), 
            dc_id: Default::default(), 
            operator_id: Default::default(), 
            subnet_id: Default::default(), 
            node_provider_id: Default::default() 
        }
    }
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

mod tests {
    use std::{collections::BTreeSet, str::FromStr};
    use ic_types::{NodeId, PrincipalId, SubnetId};
    use service_discovery::TargetGroup;

    use crate::server_handlers::filters::AttributesFilter;

    #[test]
    fn attributes_filter_test() {
        let target_group = TargetGroup {
            node_id: NodeId::from(PrincipalId::new_anonymous()),
            ic_name: "mercury".into(),
            targets: BTreeSet::new(),
            subnet_id: Some(SubnetId::from(
                PrincipalId::from_str("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae").unwrap()
            )),
            dc_id: "test".into(),
            operator_id: PrincipalId::new_anonymous(),
            node_provider_id: PrincipalId::new_anonymous(),
        };

        let filter = AttributesFilter{
            ic_name: Some("mercury".into()),
            ..Default::default()
        };
        assert!(filter.filter(&target_group));

        let filter = AttributesFilter{
            ic_name: Some("other_ic".into()),
            ..Default::default()
        };
        assert!(!filter.filter(&target_group));

        let filter = AttributesFilter{
            subnet_id: Some(SubnetId::from(
                PrincipalId::from_str("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae").unwrap()
            )),
            ..Default::default()
        };
        assert!(filter.filter(&target_group));

        let filter = AttributesFilter{
            subnet_id: Some(SubnetId::from(PrincipalId::new_anonymous())),
            ..Default::default()
        };
        assert!(!filter.filter(&target_group));

        let filter = AttributesFilter{
            ic_name: Some("mercury".into()),
            subnet_id: Some(SubnetId::from(
                PrincipalId::from_str("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae").unwrap()
            )),
            ..Default::default()
        };
        assert!(filter.filter(&target_group));

        let filter = AttributesFilter{
            ic_name: Some("mercury".into()),
            subnet_id: Some(SubnetId::from(PrincipalId::new_anonymous())),
            ..Default::default()
        };
        assert!(!filter.filter(&target_group));
    }
}
