use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use mercury_management_types::{requests::MembershipReplaceRequest, TopologyProposal};

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

    pub async fn membership_replace(&self, request: MembershipReplaceRequest) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(
                self.url
                    .join("subnet/membership/replace")
                    .map_err(|e| anyhow::anyhow!(e))?,
            )
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .json::<decentralization::SubnetChangeResponse>()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
