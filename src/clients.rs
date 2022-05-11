use async_trait::async_trait;
use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use mercury_management_types::{requests::MembershipReplaceRequest, TopologyProposal};
use serde::de::DeserializeOwned;

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
            .rest_send()
            .await
    }

    pub async fn membership_replace(&self, request: MembershipReplaceRequest) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(
                self.url
                    .join("subnet/membership/replace")
                    .map_err(|e| anyhow::anyhow!(e))?,
            )
            .json(&request)
            .rest_send()
            .await
    }
}

#[async_trait]
trait RESTRequestBuilder {
    async fn rest_send<T: DeserializeOwned>(self) -> anyhow::Result<T>;
}

#[async_trait]
impl RESTRequestBuilder for reqwest::RequestBuilder {
    async fn rest_send<T: DeserializeOwned>(self) -> anyhow::Result<T> {
        let response_result = self.send().await?;
        if let Err(e) = response_result.error_for_status_ref() {
            Err(anyhow::anyhow!(
                "failed request (error: {}, response: {})",
                e,
                response_result.text().await?
            ))
        } else {
            response_result
                .text()
                .await
                .map_err(|e| anyhow::anyhow!(e))
                .and_then(|body| {
                    serde_json::from_str::<T>(&body).map_err(|e| {
                        anyhow::anyhow!(
                            "Error decoding {} from backend output: {}\n{}",
                            std::any::type_name::<T>(),
                            body,
                            e
                        )
                    })
                })
        }
    }
}
