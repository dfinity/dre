use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use mercury_management_types::TopologyProposal;

#[derive(Clone)]
pub struct DashboardBackendClient {
    pub url: reqwest::Url,
}

impl DashboardBackendClient {
    pub async fn subnet_pending_action(&self, subnet: PrincipalId) -> anyhow::Result<Option<TopologyProposal>> {
        reqwest::Client::new()
            .get(
                self.url
                    .join(&format!("subnet/{subnet}/pending_action"))
                    .map_err(|e| anyhow::anyhow!(e))?,
            )
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .json::<Option<TopologyProposal>>()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}

#[derive(Clone)]
pub struct DecentralizationClient {
    pub url: reqwest::Url,
}

impl DecentralizationClient {
    pub async fn replace(&self, nodes: &[PrincipalId]) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(self.url.join("replace").map_err(|e| anyhow::anyhow!(e))?)
            .json(&decentralization::ReplaceRequest { nodes: nodes.to_vec() })
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .json::<decentralization::SubnetChangeResponse>()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn optimize(
        &self,
        subnet: PrincipalId,
        query: decentralization::OptimizeQuery,
    ) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(
                self.url
                    .join(&format!("{subnet}/optimize"))
                    .map_err(|e| anyhow::anyhow!(e))?,
            )
            .json(&query)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .json::<decentralization::SubnetChangeResponse>()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
