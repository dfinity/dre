use std::{
    collections::BTreeMap,
    fmt::Display,
    path::{Path, PathBuf},
    time::Duration,
};

use futures::{future::BoxFuture, TryFutureExt};
use ic_types::PrincipalId;
use itertools::Itertools;
use log::warn;
use mockall::automock;
use regex::Regex;
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;

#[automock]
pub trait DiscourseClient: Sync + Send {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, body: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>>;

    fn create_authorized_subnets_update_forum_post(&self, body: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>>;

    fn add_proposal_url_to_post(&self, post_id: Option<u64>, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>>;
}

pub struct DiscourseClientImp {
    client: Client,
    forum_url: String,
    api_key: String,
    api_user: String,
    offline: bool,
    skip_forum_post_creation: bool,
    subnet_topic_file_override: Option<PathBuf>,
}

impl DiscourseClientImp {
    pub fn new(
        url: String,
        api_key: String,
        api_user: String,
        offline: bool,
        skip_forum_post_creation: bool,
        subnet_topic_file_override: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            forum_url: url,
            api_key,
            api_user,
            offline,
            skip_forum_post_creation,
            subnet_topic_file_override,
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
        let response: serde_json::Value = self
            .request("categories.json?include_subcategories=true".to_string(), Method::GET, None)
            .await?;

        let categories = response
            .get("category_list")
            .ok_or(anyhow::anyhow!("Expected `category_list` to be in the response"))?
            .as_object()
            .ok_or(anyhow::anyhow!("Expected `category_list` to be an object"))?
            .get("categories")
            .ok_or(anyhow::anyhow!("Expected `categories` to be in the response"))?
            .as_array()
            .ok_or(anyhow::anyhow!("Expected `categories` to be an array"))?;

        let categories = serde_json::from_value::<Vec<CategoryResponse>>(serde_json::to_value(categories)?)?;

        categories
            .iter()
            .find_map(|category| category.contains(&category_name).map(|category| category.id))
            .ok_or(anyhow::anyhow!("Failed to find category with name `{}`", category_name))
    }

    async fn create_topic(&self, topic: DiscourseTopic) -> anyhow::Result<DiscourseResponse> {
        let category = self.get_category_id(topic.category).await?;

        let payload = json!({
           "title": topic.title,
           "category": category,
           "raw": topic.content,
           "tags": topic.tags
        });
        let payload = serde_json::to_string(&payload)?;

        let topic: serde_json::Value = self
            .request("posts.json?skip_validations=true".to_string(), Method::POST, Some(payload))
            .await?;

        let (id, topic_slug, topic_id) = match (topic.get("id"), topic.get("topic_slug"), topic.get("topic_id")) {
            (Some(id), Some(topic_slug), Some(topic_id)) => (id.as_u64().unwrap(), topic_slug.as_str().unwrap(), topic_id.as_u64().unwrap()),
            _ => anyhow::bail!("Expected to get `id`, `topic_slug` and `topic_id` while creating topic"),
        };

        Ok(DiscourseResponse {
            update_id: Some(id),
            url: self.format_topic_url(topic_slug, topic_id),
        })
    }

    fn format_topic_url(&self, topic_slug: &str, topic_id: u64) -> String {
        format!("{}/t/{}/{}", self.forum_url, topic_slug, topic_id)
    }

    async fn create_post(&self, content: String, topic_id: u64) -> anyhow::Result<DiscourseResponse> {
        let payload = json!({
           "raw": content,
           "topic_id": topic_id
        });
        let payload = serde_json::to_string(&payload)?;

        let post: serde_json::Value = self
            .request("posts.json?skip_validations=true".to_string(), Method::POST, Some(payload))
            .await?;

        let (id, topic_slug, post_number) = match (post.get("id"), post.get("topic_slug"), post.get("post_number")) {
            (Some(id), Some(topic_slug), Some(post_number)) => (id.as_u64().unwrap(), topic_slug.as_str().unwrap(), post_number.as_u64().unwrap()),
            _ => anyhow::bail!("Expected to get `id`, `topic_slug` and `post_number` while creating topic"),
        };

        Ok(DiscourseResponse {
            update_id: Some(id),
            url: self.format_post_url(topic_slug, topic_id, post_number),
        })
    }

    fn format_post_url(&self, topic_slug: &str, topic_id: u64, post_number: u64) -> String {
        format!("{}/{}", self.format_topic_url(topic_slug, topic_id), post_number)
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

    fn request_from_user_topic(&self, err: anyhow::Error, topic: DiscourseTopic) -> anyhow::Result<Option<DiscourseResponse>> {
        warn!("Received error: {:?}", err);
        warn!("Please create a topic with the following information");
        println!("{}", topic);
        let forum_post_link = dialoguer::Input::<String>::new()
            .with_prompt("Forum post link")
            .allow_empty(true)
            .interact()?;
        Ok(Some(DiscourseResponse {
            url: forum_post_link,
            update_id: None,
        }))
    }

    fn request_from_user_post(&self, err: anyhow::Error, body: String, topic_url: String) -> anyhow::Result<Option<DiscourseResponse>> {
        warn!("Received error: {:?}", err);
        warn!("Please create a post in topic {} with the following content", topic_url);
        println!("{}", body);
        let forum_post_link = dialoguer::Input::<String>::new()
            .with_prompt("Forum post link")
            .allow_empty(true)
            .interact()?;
        Ok(Some(DiscourseResponse {
            url: forum_post_link,
            update_id: None,
        }))
    }
}

const NNS_PROPOSAL_DISCUSSION: &str = "NNS proposal discussions";
const SUBNET_MANAGEMENT_TAG: &str = "Subnet-management";

#[derive(Debug, Deserialize, Clone)]
struct CategoryResponse {
    id: u64,
    name: String,
    subcategory_list: Option<Vec<CategoryResponse>>,
}

impl CategoryResponse {
    fn contains(&self, category_name: &str) -> Option<&Self> {
        if self.name == category_name {
            return Some(self);
        }

        if let Some(subcategories) = &self.subcategory_list {
            for subcategory in subcategories {
                if let Some(category) = subcategory.contains(category_name) {
                    return Some(category);
                }
            }
        }

        None
    }
}

#[derive(Deserialize)]
struct SubnetTopicInfo {
    slug: String,
    topic_id: u64,
}

const SUBNET_TOPICS_AND_SLUGS: &str = include_str!("assets/subnet_topic_map.json");
fn get_subnet_topics_map() -> BTreeMap<PrincipalId, SubnetTopicInfo> {
    serde_json::from_str(SUBNET_TOPICS_AND_SLUGS).unwrap()
}

fn get_subnet_topics_from_path(path: &Path) -> anyhow::Result<BTreeMap<PrincipalId, SubnetTopicInfo>> {
    let file = std::fs::File::open(path)?;
    serde_json::from_reader(file).map_err(anyhow::Error::from)
}

impl DiscourseClient for DiscourseClientImp {
    fn create_replace_nodes_forum_post(&self, subnet_id: PrincipalId, body: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>> {
        Box::pin(async move {
            if self.offline || self.skip_forum_post_creation {
                return Ok(None);
            }

            let subnet_topic_map = match &self.subnet_topic_file_override {
                Some(path) => get_subnet_topics_from_path(path)?,
                None => get_subnet_topics_map(),
            };
            let topic_info = subnet_topic_map.get(&subnet_id).ok_or(anyhow::anyhow!(
                "Subnet {} not found in the subnet topic map. Don't know where to create a forum post",
                subnet_id.to_string()
            ))?;

            self.create_post(body.clone(), topic_info.topic_id)
                .await
                .map(Some)
                .or_else(|e| self.request_from_user_post(e, body, self.format_topic_url(&topic_info.slug, topic_info.topic_id)))
        })
    }

    fn add_proposal_url_to_post(&self, post_id: Option<u64>, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            if self.offline || self.skip_forum_post_creation {
                return Ok(());
            }

            if post_id.is_none() {
                warn!("Failed to find post id to update the proposal id");
                return Ok(());
            }
            let post_id = post_id.unwrap();

            let orig_content = self.get_post_content(post_id).await?;
            let new_content = format!(
                r#"A new proposal with id [{0}](https://dashboard.internetcomputer.org/proposal/{0}) has been submitted, please take a look.

[details="Click here to open proposal details"]
{1}
[/details]
"#,
                proposal_id, orig_content
            );
            self.update_post_content(post_id, new_content).await
        })
    }

    fn create_authorized_subnets_update_forum_post(&self, body: String) -> BoxFuture<'_, anyhow::Result<Option<DiscourseResponse>>> {
        let post = DiscourseTopic {
            title: "Updating the list of public subnets".to_string(),
            content: body,
            tags: vec![SUBNET_MANAGEMENT_TAG.to_string()],
            category: NNS_PROPOSAL_DISCUSSION.to_string(),
        };
        let post_clone = post.clone();

        let try_call = async move {
            if self.offline || self.skip_forum_post_creation {
                return Ok(None);
            }
            let topic = self.create_topic(post).await?;
            Ok(Some(topic))
        };
        Box::pin(async move { try_call.await.or_else(|e| self.request_from_user_topic(e, post_clone)) })
    }
}

pub fn parse_proposal_id_from_ic_admin_response(response: String) -> anyhow::Result<u64> {
    // To ensure we capture just the line with "proposal xyz"
    let last_line = response
        .lines()
        .filter(|line| !line.trim().is_empty())
        .last()
        .ok_or(anyhow::anyhow!("Expected at least one line in the response"))?;
    let re = Regex::new(r"\s*(\d+)\s*")?;

    re.captures(&last_line.to_lowercase())
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
    pub update_id: Option<u64>,
}

#[derive(Clone)]
pub struct DiscourseTopic {
    title: String,
    content: String,
    tags: Vec<String>,
    category: String,
}

impl Display for DiscourseTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"Title: {}
Category: {}
Tags: [{}]
Content:
{}"#,
            self.title,
            self.category,
            self.tags.iter().join(", "),
            self.content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proposal_id_test() {
        let text = r#" some text blah 111
proposal 123456

"#
        .to_string();
        let parsed = parse_proposal_id_from_ic_admin_response(text).unwrap();
        assert_eq!(parsed, 123456);

        let text = "222222".to_string();
        let parsed = parse_proposal_id_from_ic_admin_response(text).unwrap();
        assert_eq!(parsed, 222222);

        let text = "Proposal id 123456".to_string();
        let parsed = parse_proposal_id_from_ic_admin_response(text).unwrap();
        assert_eq!(parsed, 123456)
    }
}
