use std::time::Duration;

use futures::{future::BoxFuture, TryFutureExt};
use ic_types::PrincipalId;
use mockall::automock;
use regex::Regex;
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde_json::json;

#[automock]
pub trait DiscourseClient: Sync + Send {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, summary: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>>;

    fn add_proposal_url_to_post(&self, post_id: u64, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>>;
}

pub struct DiscourseClientImp {
    client: Client,
    forum_url: String,
    api_key: String,
    api_user: String,
    offline: bool,
}

impl DiscourseClientImp {
    pub fn new(url: String, api_key: String, api_user: String, offline: bool) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            forum_url: url,
            api_key,
            api_user,
            offline,
        })
    }

    async fn request<T: DeserializeOwned>(&self, url: String, method: Method, payload: Option<String>) -> anyhow::Result<T> {
        let mut request = self
            .client
            .request(method.clone(), format!("{}/{}", self.forum_url, url))
            .header("Api-Key", &self.api_key)
            .header("Api-Username", &self.api_user)
            .header("Content-Type", "application/json");

        if method == Method::POST || method == Method::PUT {
            let payload = payload.ok_or(anyhow::anyhow!("Expected payload for `{}` method", method))?;
            request = request.body(payload);
        }

        request.send().await?.error_for_status()?.json().map_err(anyhow::Error::from).await
    }

    async fn get_category_id(&self, category_name: String) -> anyhow::Result<u64> {
        let response: serde_json::Value = self.request("categories.json".to_string(), Method::GET, None).await?;

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
                    None
                }
                _ => None,
            })
            .ok_or(anyhow::anyhow!("Failed to find category with name `{}`", category_name))
    }

    async fn create_topic(&self, title: String, summary: String, category: String, tags: Vec<String>) -> anyhow::Result<DiscourseResponse> {
        let category = self.get_category_id(category).await?;

        let payload = json!({
           "title": title,
           "category": category,
           "raw": summary,
           "tags": tags
        });
        let payload = serde_json::to_string(&payload)?;

        let topic: serde_json::Value = self
            .request("posts.json?skip_validations=true".to_string(), Method::POST, Some(payload))
            .await?;

        let (id, topic_slug, topic_id) = match (topic.get("id"), topic.get("topic_slug"), topic.get("topic_id")) {
            (Some(id), Some(topic_slug), Some(topic_id)) => (id.as_u64().unwrap(), topic_slug.as_str().unwrap(), topic_id.as_u64().unwrap()),
            _ => anyhow::bail!("Expected to get `id` and `topic_id` while creating topic"),
        };

        Ok(DiscourseResponse {
            id,
            url: format!("{}/t/{}/{}", self.forum_url, topic_slug, topic_id),
        })
    }

    async fn get_post_content(&self, post_id: u64) -> anyhow::Result<String> {
        let post: serde_json::Value = self.request(format!("posts/{}.json", post_id), Method::GET, None).await?;
        let content = post
            .get("raw")
            .ok_or(anyhow::anyhow!("Expected post response to container `raw` in the body"))?
            .as_str()
            .ok_or(anyhow::anyhow!("Expected `raw` to be of type `String`"))?;
        Ok(content.to_string())
    }

    async fn update_post_content(&self, post_id: u64, new_content: String) -> anyhow::Result<()> {
        let payload = json!({
            "post": {
                "raw": new_content
            }
        });
        let payload = serde_json::to_string(&payload)?;

        self.request::<serde_json::Value>(format!("posts/{}.json", post_id), Method::PUT, Some(payload))
            .await
            .map(|_resp| ())
    }
}

impl DiscourseClient for DiscourseClientImp {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, summary: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>> {
        Box::pin(async move {
            if self.offline {
                return Ok(None);
            }
            let subnet_id = subnet_id.to_string();
            let (first_part, _rest) = subnet_id
                .split_once("-")
                .ok_or(anyhow::anyhow!("Unexpected principal format `{}`", subnet_id))?;
            let topic = self
                .create_topic(
                    format!("Replacing nodes in subnet {}", first_part),
                    summary,
                    "Governance".to_string(),
                    vec!["Subnet-management".to_string()],
                )
                .await?;
            Ok(Some(topic))
        })
    }

    fn add_proposal_url_to_post(&self, post_id: u64, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            if self.offline {
                return Ok(());
            }

            let content = self.get_post_content(post_id).await?;
            let new_content = format!(
                r#"{0}

Proposal id [{1}](https://dashboard.internetcomputer.org/proposal/{1})"#,
                content, proposal_id
            );
            self.update_post_content(post_id, new_content).await
        })
    }
}

pub fn parse_proposal_id_from_governance_response(response: String) -> anyhow::Result<u64> {
    let re = Regex::new(r"proposal\s+(\d+)")?;

    re.captures(&response.to_lowercase())
        .ok_or(anyhow::anyhow!("Expected some captures while parsing id from governance canister"))?
        .iter()
        .last()
        .ok_or(anyhow::anyhow!(
            "Expected at least one captures while parsing id from governance canister"
        ))?
        .ok_or(anyhow::anyhow!("Expected last element to be of type `Some()`"))?
        .as_str()
        .parse()
        .map_err(anyhow::Error::from)
}

#[derive(Debug)]
pub struct DiscourseResponse {
    pub url: String,
    pub id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proposal_id_test() {
        let text = "propoSAL 123456".to_string();
        let parsed = parse_proposal_id_from_governance_response(text).unwrap();
        assert_eq!(parsed, 123456);

        let text = "Proposal 222222".to_string();
        let parsed = parse_proposal_id_from_governance_response(text).unwrap();
        assert_eq!(parsed, 222222);

        let text = "Proposal id 123456".to_string();
        let parsed = parse_proposal_id_from_governance_response(text);
        assert!(parsed.is_err())
    }
}
