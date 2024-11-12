use std::time::Duration;

use futures::future::BoxFuture;
use ic_protobuf::types::v1::PrincipalId;
use mockall::automock;
use reqwest::Client;

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

    async fn get_category_id(&self, category: String) -> anyhow::Result<u8> {
        Ok(0)
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
    async fn get_categories() {
        let client = get_client();

        let category = client.get_category_id("governance".to_string()).await.unwrap();

        println!("{}", category)
    }
}
