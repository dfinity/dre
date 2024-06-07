use async_trait::async_trait;
use decentralization::HealResponse;
use decentralization::SubnetChangeResponse;
use ic_base_types::PrincipalId;
use ic_management_types::{
    requests::{HealRequest, MembershipReplaceRequest, NodesRemoveRequest, NodesRemoveResponse, SubnetCreateRequest, SubnetResizeRequest},
    Artifact, Network, NetworkError, Release, TopologyProposal,
};
use log::error;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct DashboardBackendClient {
    pub(crate) url: reqwest::Url,
}

impl DashboardBackendClient {
    // Only used in tests, which should be cleaned up together with this code.
    #[allow(dead_code)]
    pub fn new(network: &Network, dev: bool) -> DashboardBackendClient {
        Self {
            url: reqwest::Url::parse(if !dev {
                "https://dashboard.internal.dfinity.network/"
            } else {
                "http://localhost:17000/"
            })
            .expect("invalid base url")
            .join("api/proxy/registry/")
            .expect("failed to join url")
            .join(&network.name)
            .expect("failed to join url"),
        }
    }

    pub fn new_with_backend_url(url: String) -> Self {
        Self {
            url: reqwest::Url::parse(&url).unwrap(),
        }
    }

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
            .post(self.url.join("subnet/membership/replace").map_err(|e| anyhow::anyhow!(e))?)
            .json(&request)
            .rest_send()
            .await
    }

    pub async fn subnet_resize(&self, request: SubnetResizeRequest) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(self.url.join("subnet/membership/resize").map_err(|e| anyhow::anyhow!(e))?)
            .json(&request)
            .rest_send()
            .await
    }

    pub async fn subnet_create(&self, request: SubnetCreateRequest) -> anyhow::Result<SubnetChangeResponse> {
        reqwest::Client::new()
            .post(self.url.join("subnet/create").map_err(|e| anyhow::anyhow!(e))?)
            .json(&request)
            .rest_send()
            .await
    }

    pub async fn get_retireable_versions(&self, release_artifact: &Artifact) -> anyhow::Result<Vec<Release>> {
        reqwest::Client::new()
            .get(
                self.url
                    .join(&format!("release/retireable/{}", release_artifact))
                    .map_err(|e| anyhow::anyhow!(e))?,
            )
            .rest_send()
            .await
    }

    pub async fn get_nns_replica_version(&self) -> anyhow::Result<String> {
        reqwest::Client::new()
            .get(self.url.join("release/versions/nns").map_err(|e| anyhow::anyhow!(e))?)
            .rest_send()
            .await
    }

    pub async fn remove_nodes(&self, request: NodesRemoveRequest) -> anyhow::Result<NodesRemoveResponse> {
        reqwest::Client::new()
            .post(self.url.join("nodes/remove").map_err(|e| anyhow::anyhow!(e))?)
            .json(&request)
            .rest_send()
            .await
    }

    pub(crate) async fn network_heal(&self, request: HealRequest) -> anyhow::Result<HealResponse> {
        reqwest::Client::new()
            .post(self.url.join("network/heal").map_err(|e| anyhow::anyhow!(e))?)
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
            let response = response_result.text().await?;
            match serde_json::from_str(&response) {
                Ok(NetworkError::ResizeFailed(s)) => {
                    error!("{}", s);
                    Err(anyhow::anyhow!("failed request (error: {})", e))
                }
                _ => Err(anyhow::anyhow!("failed request (error: {}, response: {})", e, response)),
            }
        } else {
            response_result.text().await.map_err(|e| anyhow::anyhow!(e)).and_then(|body| {
                serde_json::from_str::<T>(&body)
                    .map_err(|e| anyhow::anyhow!("Error decoding {} from backend output: {}\n{}", std::any::type_name::<T>(), body, e))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dashboard_backend_client_url() {
        let mainnet = Network::new("mainnet", &vec![]).await.expect("failed to create mainnet network");
        let staging = Network::new("staging", &vec![]).await.expect("failed to create staging network");
        assert_eq!(
            DashboardBackendClient::new(&mainnet, false).url.to_string(),
            "https://dashboard.internal.dfinity.network/api/proxy/registry/mainnet"
        );
        assert_eq!(
            DashboardBackendClient::new(&staging, false).url.to_string(),
            "https://dashboard.internal.dfinity.network/api/proxy/registry/staging"
        );
        assert_eq!(
            DashboardBackendClient::new(&mainnet, true).url.to_string(),
            "http://localhost:17000/api/proxy/registry/mainnet"
        );
        assert_eq!(
            DashboardBackendClient::new(&staging, true).url.to_string(),
            "http://localhost:17000/api/proxy/registry/staging"
        );
    }
}
