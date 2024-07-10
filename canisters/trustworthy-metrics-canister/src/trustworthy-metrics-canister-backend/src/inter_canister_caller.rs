use dfn_core::api::PrincipalId;
use ic_management_canister_types::{NodeMetrics, NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use itertools::Itertools;



pub struct InterCanisterCaller {}
impl InterCanisterCaller {
    pub  fn new() -> Self {
        Self{}
    }
    async fn get_subnets(&self) -> anyhow::Result<Vec<PrincipalId>> {

        let (registry_subnets, _): (SubnetListRecord, _) = ic_nns_common::registry::get_value("subnet_list".as_bytes(), None).await?;
        let subnets = registry_subnets.subnets
            .into_iter()
            .map(|subnet_id: Vec<u8>| {
                PrincipalId::try_from(subnet_id).unwrap()
            })
            .collect_vec();

        ic_cdk::println!("got {} subnets", subnets.len());

        anyhow::Ok(subnets)
    }

    async fn get_node_metrics_history(&self, subnet_id: PrincipalId) -> anyhow::Result<(PrincipalId, Vec<NodeMetrics>)> {
        let contract = NodeMetricsHistoryArgs {
            subnet_id,
            start_at_timestamp_nanos: 0
        };
        
        let node_metrics_result = ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            candid::Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128
        )
        .await
        .map(|(result_ok,)| {
            let node_metrics = result_ok
                .into_iter()
                .flat_map(|r| r.node_metrics)
                .collect_vec();

            (subnet_id, node_metrics)
        });

        node_metrics_result.map_err(|(code, msg)| anyhow::anyhow!(
            "Error when calling management canister:\n Code:{:?}\nMsg:{}",
            code, msg
        ))
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let subnets = self.get_subnets().await?;
        let subnets_metrics = subnets
            .into_iter()
            .map(|subnet_id: PrincipalId| self.get_node_metrics_history(subnet_id))
            .collect_vec(); 

        let subnet_with_node_metrics = futures::future::try_join_all(subnets_metrics).await?;
        let node_metrics = subnet_with_node_metrics.into_iter().flat_map(|(_, metrics)| metrics).collect_vec();

        ic_cdk::println!("Collected {:?} metrics", node_metrics.len());

        anyhow::Ok(())
    }
}


