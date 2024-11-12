use std::time::Duration;

use futures::{future::BoxFuture, TryFutureExt};
use ic_protobuf::types::v1::PrincipalId;
use mockall::automock;
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;

#[automock]
pub trait DiscourseClient: Sync + Send {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, summary: String) -> BoxFuture<'_, anyhow::Result<DiscourseResponse>>;

    fn add_proposal_url_to_post(&self, id: String, proposal_url: String) -> BoxFuture<'_, anyhow::Result<()>>;
}

pub struct DiscourseClientImp {
    client: Client,
    forum_url: String,
    api_key: String,
}

impl DiscourseClientImp {
    pub fn new(url: String, api_key: String) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            forum_url: url,
            api_key,
        })
    }

    async fn request<T: DeserializeOwned>(&self, url: String, method: Method) -> anyhow::Result<T> {
        self.client
            .request(method, format!("{}/{}", self.forum_url, url))
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .map_err(anyhow::Error::from)
            .await
    }

    async fn get_category_id(&self, category_name: String) -> anyhow::Result<u64> {
        let response: serde_json::Value = self.request("categories.json".to_string(), Method::GET).await?;

        let categories = response
            .get("category_list")
            .ok_or(anyhow::anyhow!("Expected `category_list` to be in the response"))?
            .as_object()
            .ok_or(anyhow::anyhow!("Expected `category_list` to be an object"))?
            .get("categories")
            .ok_or(anyhow::anyhow!("Expected `categories` to be in the response"))?
            .as_array()
            .ok_or(anyhow::anyhow!("Expected `categories` to be an array"))?;

        categories
            .iter()
            .find_map(|category| match (category.get("id"), category.get("name")) {
                (Some(id), Some(name)) => {
                    let name = name.as_str().unwrap();
                    let id = id.as_u64().unwrap();
                    if name == category_name {
                        return Some(id);
                    }
                    return None;
                }
                _ => None,
            })
            .ok_or(anyhow::anyhow!("Failed to find category with name `{}`", category_name))
    }

    async fn create_post(&self, title: String, summary: String, category: String, tags: Vec<String>) -> anyhow::Result<DiscourseResponse> {
        Ok(DiscourseResponse {
            id: "123".to_string(),
            url: "123".to_string(),
        })
    }
}

impl DiscourseClient for DiscourseClientImp {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, summary: String) -> BoxFuture<'_, anyhow::Result<DiscourseResponse>> {
        todo!()
    }

    fn add_proposal_url_to_post(&self, id: String, proposal_url: String) -> BoxFuture<'_, anyhow::Result<()>> {
        todo!()
    }
}

pub struct DiscourseResponse {
    pub url: String,
    pub id: String,
}

struct CreateTopicPayload {
    title: String,
    raw: String,
    category: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_client() -> DiscourseClientImp {
        DiscourseClientImp::new(
            "http://localhost:3000".to_string(),
            "37e522a546506f9e3751265669de2576896491b0a3c39d0524be8689736c8722".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn discourse_test() {
        let client = get_client();

        let category = client.get_category_id("Governance".to_string()).await.unwrap();

        println!("{}", category)
    }
}
