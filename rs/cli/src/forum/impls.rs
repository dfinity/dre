use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use futures::{future::BoxFuture, FutureExt};
use ic_types::PrincipalId;
use itertools::Itertools;
use log::{info, warn};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use url::Url;

use super::{ForumParameters, ForumPost, ForumPostHandler, ForumPostKind};

/// Type of "forum post client" for user-supplied links.
pub(crate) struct UserSuppliedLink {
    pub(crate) url: Option<url::Url>,
}

/// This forum post type does not create or update anything.
impl ForumPostHandler for UserSuppliedLink {
    fn forum_post(&self, _kind: ForumPostKind) -> BoxFuture<'_, anyhow::Result<Box<dyn ForumPost>>> {
        FutureExt::boxed(async { Ok(Box::new(OptionalFixedLink { url: self.url.clone() }) as Box<dyn ForumPost>) })
    }
}

/// Type of "forum post" for user-supplied links (or omitted links).
struct OptionalFixedLink {
    url: Option<url::Url>,
}

/// This forum post type does not update anything, only returns the user-supplied link (if any).
impl ForumPost for OptionalFixedLink {
    fn url(&self) -> Option<url::Url> {
        self.url.clone()
    }
    fn add_proposal_url(&self, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>> {
        if let Some(u) = &self.url {
            info!(
                "Please manually cite the proposal URL https://dashboard.internetcomputer.org/proposal/{} in the contents of the link {} you supplied earlier",
                proposal_id, u
            );
        }
        FutureExt::boxed(async { Ok(()) })
    }
}

/// A "forum post client" that just interactively prompts the user for a link.
pub(crate) struct Prompter {}

impl Prompter {
    fn get_url_from_user(&self) -> BoxFuture<'_, anyhow::Result<Box<dyn ForumPost>>> {
        FutureExt::boxed(async {
            let forum_post_link = dialoguer::Input::<String>::new()
                .with_prompt("Forum post link")
                .allow_empty(false)
                .interact()?;
            Ok(Box::new(OptionalFixedLink {
                url: Some(url::Url::from_str(forum_post_link.as_str())?),
            }) as Box<dyn ForumPost>)
        })
    }
}

/// This forum post type does not update anything because the user interactively supplied a link.
impl ForumPostHandler for Prompter {
    fn forum_post(&self, _kind: ForumPostKind) -> BoxFuture<'_, anyhow::Result<Box<dyn ForumPost>>> {
        self.get_url_from_user()
    }
}

/// Type of forum post client that manages the creation and update of forum posts on Discourse.
pub(crate) struct Discourse {
    client: DiscourseClientImp,
    simulate: bool,
    skip_forum_post_creation: bool,
    subnet_topic_file_override: Option<PathBuf>,
}

impl Discourse {
    pub(crate) fn new(forum_opts: ForumParameters, simulate: bool) -> anyhow::Result<Self> {
        // FIXME: move me to the DiscourseClientImp struct.
        let placeholder_key = "placeholder_key".to_string();
        let placeholder_user = "placeholder_user".to_string();
        let placeholder_url = "https://placeholder_url.com".to_string();

        let (api_key, api_user, forum_url) = match (
            forum_opts.discourse_api_key.clone(),
            forum_opts.discourse_api_user.clone(),
            forum_opts.discourse_api_url.clone(),
        ) {
            // Actual api won't be called so these values don't matter
            _ if forum_opts.discourse_skip_post_creation => (placeholder_key, placeholder_user, placeholder_url),
            (api_key, api_user, forum_url) => (
                api_key.unwrap_or_else(|| {
                    warn!("Will use placeholder_key for discourse api key since it was not provided");
                    placeholder_key
                }),
                api_user.unwrap_or_else(|| {
                    warn!("Will use placeholder_user for discourse api user since it was not provided");
                    placeholder_user
                }),
                forum_url,
            ),
        };

        Ok(Self {
            client: DiscourseClientImp::new(forum_url, api_key, api_user)?,
            // `simulate` for discourse client means that it shouldn't try and create posts or update them.
            simulate,
            skip_forum_post_creation: forum_opts.discourse_skip_post_creation,
            subnet_topic_file_override: forum_opts.discourse_subnet_topic_override_file_path.clone(),
        })
    }

    async fn request_from_user_topic_or_post(&self) -> anyhow::Result<DiscourseResponse> {
        // FIXME: this should move to caller.
        let forum_post_link = dialoguer::Input::<String>::new()
            .with_prompt("Forum post link")
            .allow_empty(false)
            .interact()?;

        let (update_id, is_topic) = self.get_post_update_id_from_url(forum_post_link.as_str()).await?;
        Ok(DiscourseResponse {
            url: forum_post_link,
            update_id,
            is_topic,
        })
    }

    async fn get_post_update_id_from_url(&self, url: &str) -> anyhow::Result<(u64, bool)> {
        let topic_and_post_number_re = regex::Regex::new("/([0-9])+/([0-9])+(/|)$").unwrap();
        let topic_re = regex::Regex::new("/([0-9])+(/|)$").unwrap();

        let (topic_id, post_number, is_topic) = if let Some(captures) = topic_and_post_number_re.captures(url) {
            (
                u64::from_str(captures.get(1).unwrap().as_str()).unwrap(),
                u64::from_str(captures.get(2).unwrap().as_str()).unwrap(),
                false,
            )
        } else if let Some(captures) = topic_re.captures(url) {
            (u64::from_str(captures.get(1).unwrap().as_str()).unwrap(), 1, true)
        } else {
            return Err(anyhow::anyhow!(
                "The provided URL does not have any topic or post ID this tool can use to locate the post for later editing.",
            ));
        };
        Ok((self.client.get_post_id_for_topic_and_post_number(topic_id, post_number).await?, is_topic))
    }

    async fn request_from_user_topic(&self, err: Option<anyhow::Error>, topic: DiscourseTopic) -> anyhow::Result<DiscourseResponse> {
        // FIXME: this should move to caller.
        if let Some(e) = err {
            warn!("While creating a new topic, Discourse returned an error: {:?}", e);
        }
        let url = self.client.format_url_for_automatic_topic_creation(topic)?;

        warn!("Please create a topic on the following link: {}", url);

        let forum_post_link = dialoguer::Input::<String>::new()
            .with_prompt("Forum post link")
            .allow_empty(false)
            .interact()?;

        let (update_id, is_topic) = self.get_post_update_id_from_url(forum_post_link.as_str()).await?;
        Ok(DiscourseResponse {
            url: forum_post_link,
            update_id,
            is_topic,
        })
    }

    async fn request_from_user_post(&self, err: Option<anyhow::Error>, body: String, topic_url: String) -> anyhow::Result<DiscourseResponse> {
        // FIXME: this should move to caller.
        if let Some(e) = err {
            warn!("While creating a new post in topic {}, Discourse returned an error: {:?}", topic_url, e);
        }
        warn!("Please create a post in topic {} with the following content", topic_url);
        println!("{}", body);
        let forum_post_link = dialoguer::Input::<String>::new()
            .with_prompt("Forum post link")
            .allow_empty(false)
            .interact()?;

        let (update_id, is_topic) = self.get_post_update_id_from_url(forum_post_link.as_str()).await?;
        Ok(DiscourseResponse {
            url: forum_post_link,
            update_id,
            is_topic,
        })
    }
}

impl ForumPostHandler for Discourse {
    fn forum_post(&self, kind: ForumPostKind) -> BoxFuture<'_, anyhow::Result<Box<dyn ForumPost>>> {
        if self.simulate {
            info!("Not creating any forum post because simulation was requested (perhaps offline or dry-run mode)");
            return FutureExt::boxed(async { Ok(Box::new(OptionalFixedLink { url: None }) as Box<dyn ForumPost>) });
        }
        if self.skip_forum_post_creation {
            info!("Not creating any forum post because user requested to skip creating forum post.");
            return FutureExt::boxed(async { Ok(Box::new(OptionalFixedLink { url: None }) as Box<dyn ForumPost>) });
        }

        let client = self.client.clone();
        let subnet_topic_file_override = self.subnet_topic_file_override.clone();
        let create_topic_or_request_it = move |client: DiscourseClientImp, topic: DiscourseTopic| async {
            match client.create_topic(topic.clone()).await {
                Ok(poast) => Ok(DiscoursePost {
                    client,
                    post_url: url::Url::from_str(poast.url.as_str())?,
                    post_id: poast.update_id,
                    put_original_post_behind_details_discloser: false,
                }),
                Err(e) => {
                    let poast = self.request_from_user_topic(Some(e), topic).await?;
                    Ok(DiscoursePost {
                        client,
                        post_url: url::Url::from_str(poast.url.as_str())?,
                        post_id: poast.update_id,
                        put_original_post_behind_details_discloser: false,
                    })
                }
            }
        };
        let try_call = async move {
            let res = match kind {
                ForumPostKind::Generic => {
                    warn!("Discourse does not support creating forum posts for this kind of proposal.  Please create a post yourself and supply the link for it to be updated afterwards.");
                    let poast = self.request_from_user_topic_or_post().await?;
                    Ok(DiscoursePost {
                        client,
                        post_url: url::Url::from_str(poast.url.as_str())?,
                        post_id: poast.update_id,
                        put_original_post_behind_details_discloser: !poast.is_topic,
                    })
                }
                ForumPostKind::ReplaceNodes { subnet_id, body } => {
                    let subnet_topic_map = match &subnet_topic_file_override {
                        Some(path) => match get_subnet_topics_from_path(path) {
                            Ok(p) => p,
                            Err(e) => {
                                return Err(anyhow::anyhow!(
                                "Subnet {} not found in the specified subnet topic map file {} (error: {}). Don't know where to create a forum post",
                                subnet_id.to_string(),
                                path.display(),
                                e
                            ))
                            }
                        },
                        None => get_subnet_topics_map(),
                    };
                    let topic_info = subnet_topic_map.get(&subnet_id).ok_or(anyhow::anyhow!(
                        "Subnet {} not found in the discovered subnet topic map. Don't know where to create a forum post",
                        subnet_id.to_string()
                    ))?;
                    match self.client.create_post(body.clone(), topic_info.topic_id).await {
                        Ok(poast) => Ok::<DiscoursePost, anyhow::Error>(DiscoursePost {
                            client,
                            post_url: url::Url::from_str(poast.url.as_str())?,
                            post_id: poast.update_id,
                            put_original_post_behind_details_discloser: !poast.is_topic,
                        }),
                        Err(e) => {
                            let poast = self
                                .request_from_user_post(Some(e), body, self.client.format_topic_url(&topic_info.slug, topic_info.topic_id))
                                .await?;
                            Ok(DiscoursePost {
                                client,
                                post_url: url::Url::from_str(poast.url.as_str())?,
                                post_id: poast.update_id,
                                put_original_post_behind_details_discloser: !poast.is_topic,
                            })
                        }
                    }
                }
                ForumPostKind::AuthorizedSubnetsUpdate { body } => {
                    create_topic_or_request_it(
                        client,
                        DiscourseTopic {
                            title: "Updating the list of public subnets".to_string(),
                            content: body,
                            tags: vec![SUBNET_MANAGEMENT_TAG.to_string()],
                            category: NNS_PROPOSAL_DISCUSSION.to_string(),
                        },
                    )
                    .await
                }
                ForumPostKind::Motion { title, summary } => {
                    create_topic_or_request_it(
                        client,
                        DiscourseTopic {
                            title: title.unwrap_or("New motion".to_string()),
                            content: summary,
                            tags: vec![SUBNET_MANAGEMENT_TAG.to_string()],
                            category: NNS_PROPOSAL_DISCUSSION.to_string(),
                        },
                    )
                    .await
                }
            }?;
            Ok(Box::new(res) as Box<dyn ForumPost>)
        };
        FutureExt::boxed(try_call)
    }
}

struct DiscoursePost {
    client: DiscourseClientImp,
    post_url: url::Url,
    post_id: u64,
    put_original_post_behind_details_discloser: bool,
}

impl ForumPost for DiscoursePost {
    fn url(&self) -> Option<url::Url> {
        Some(self.post_url.clone())
    }

    fn add_proposal_url(&self, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            let orig_content = self.client.get_post_content(self.post_id).await?;
            let new_content = if self.put_original_post_behind_details_discloser {
                format!(
                    r#"A [new proposal with ID {0}](https://dashboard.internetcomputer.org/proposal/{0}) has been submitted, please take a look.

[details="Click here to open proposal details"]
{1}
[/details]
"#,
                    proposal_id, orig_content
                )
            } else {
                format!(
                    r#"{1}

[Proposal with ID {0}](https://dashboard.internetcomputer.org/proposal/{0}) has been submitted to enact the above, please take a look.
"#,
                    proposal_id, orig_content
                )
            };

            let res = self.client.update_post_content(self.post_id, new_content).await;

            if let Err(e) = res {
                warn!("While updating forum post #{}, Discourse returned an error: {:?}", self.post_id, e);
                warn!("Please update the forum post manually to reference proposal {}.", proposal_id);
            }
            Ok(())
        })
    }
}

// FIXME: implement post deletion when update of post fails.
#[derive(Clone)]
struct DiscourseClientImp {
    client: Client,
    forum_url: String,
    api_key: String,
    api_user: String,
}

#[derive(Debug)]
enum DiscourseClientImpError {
    NotFound,
    ExpectedPayload,
    DeserializeFailed(String),
    OtherReqwestError(reqwest::Error),
}

impl Display for DiscourseClientImpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DeserializeFailed(s) => write!(f, "Error deserializing response from Discourse: {}", s),
            _ => write!(f, "Unexpected error with Discourse: {:?}", self),
        }
    }
}

impl std::error::Error for DiscourseClientImpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OtherReqwestError(source) => Some(source),
            _ => None,
        }
    }
}

impl DiscourseClientImp {
    pub fn new(url: String, api_key: String, api_user: String) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            forum_url: url,
            api_key,
            api_user,
        })
    }

    async fn request<T: DeserializeOwned>(
        &self,
        url: String,
        method: Method,
        payload: Option<String>,
        page: Option<std::num::NonZero<u64>>,
    ) -> Result<T, DiscourseClientImpError> {
        let add = match page {
            None => "".to_string(),
            Some(u) => format!("?page={}", u),
        };
        let mut request = self
            .client
            .request(method.clone(), format!("{}/{}{}", self.forum_url, url, add))
            .header("Api-Key", &self.api_key)
            .header("Api-Username", &self.api_user)
            .header("Content-Type", "application/json");

        if method == Method::POST || method == Method::PUT {
            let payload = payload.ok_or(DiscourseClientImpError::ExpectedPayload)?;
            request = request.body(payload);
        }

        match request.send().await {
            Ok(r) => {
                let rstatus = r.status();
                match r.error_for_status() {
                    Ok(t) => t
                        .json()
                        .await
                        .map_err(|e| DiscourseClientImpError::DeserializeFailed(format!("Deserialization of response failed: {}", e))),
                    Err(e) => {
                        if rstatus == reqwest::StatusCode::NOT_FOUND {
                            Err(DiscourseClientImpError::NotFound)
                        } else {
                            Err(DiscourseClientImpError::OtherReqwestError(e))
                        }
                    }
                }
            }
            Err(e) => Err(DiscourseClientImpError::OtherReqwestError(e)),
        }
    }

    async fn get_category_id(&self, category_name: String) -> anyhow::Result<u64> {
        let response: serde_json::Value = self
            .request("categories.json?include_subcategories=true".to_string(), Method::GET, None, None)
            .await
            .map_err(anyhow::Error::from)?;

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
            .request("posts.json?skip_validations=true".to_string(), Method::POST, Some(payload), None)
            .await
            .map_err(anyhow::Error::from)?;

        let (id, topic_slug, topic_id) = match (topic.get("id"), topic.get("topic_slug"), topic.get("topic_id")) {
            (Some(id), Some(topic_slug), Some(topic_id)) => (id.as_u64().unwrap(), topic_slug.as_str().unwrap(), topic_id.as_u64().unwrap()),
            _ => anyhow::bail!("Expected to get `id`, `topic_slug` and `topic_id` while creating topic"),
        };

        Ok(DiscourseResponse {
            update_id: id,
            url: self.format_topic_url(topic_slug, topic_id),
            is_topic: true,
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
            .request("posts.json?skip_validations=true".to_string(), Method::POST, Some(payload), None)
            .await?;

        let (id, topic_slug, post_number) = match (post.get("id"), post.get("topic_slug"), post.get("post_number")) {
            (Some(id), Some(topic_slug), Some(post_number)) => (id.as_u64().unwrap(), topic_slug.as_str().unwrap(), post_number.as_u64().unwrap()),
            _ => anyhow::bail!("Expected to get `id`, `topic_slug` and `post_number` while creating topic"),
        };

        Ok(DiscourseResponse {
            update_id: id,
            url: self.format_post_url(topic_slug, topic_id, post_number),
            is_topic: false,
        })
    }

    fn format_post_url(&self, topic_slug: &str, topic_id: u64, post_number: u64) -> String {
        format!("{}/{}", self.format_topic_url(topic_slug, topic_id), post_number)
    }

    async fn get_post_content(&self, post_id: u64) -> anyhow::Result<String> {
        let post: serde_json::Value = self
            .request(format!("posts/{}.json", post_id), Method::GET, None, None)
            .await
            .map_err(anyhow::Error::from)?;
        let content = post
            .get("raw")
            .ok_or(anyhow::anyhow!("Expected post response to container `raw` in the body"))?
            .as_str()
            .ok_or(anyhow::anyhow!("Expected `raw` to be of type `String`"))?;
        Ok(content.to_string())
    }

    async fn get_post_id_for_topic_and_post_number(&self, topic_id: u64, post_number: u64) -> anyhow::Result<u64> {
        #[derive(Deserialize)]
        struct Post {
            id: u64,
            post_number: u64,
        }
        #[derive(Deserialize)]
        struct PostStream {
            posts: Vec<Post>,
        }
        #[derive(Deserialize)]
        struct Topic {
            post_stream: PostStream,
        }

        let mut page: std::num::NonZero<u64> = std::num::NonZero::new(1).unwrap();
        loop {
            let topic_query_result: Result<Topic, DiscourseClientImpError> =
                self.request(format!("topic/{}.json", topic_id), Method::GET, None, Some(page)).await;
            let topic = match topic_query_result {
                Err(DiscourseClientImpError::NotFound) => break,
                Err(e) => return Err(anyhow::anyhow!("Error finding post ID: {}", e)),
                Ok(topic) => topic,
            };

            for post in topic.post_stream.posts.iter() {
                if post.post_number == post_number {
                    return Ok(post.id);
                }
            }
            page = page.checked_add(1).unwrap();
        }

        Err(anyhow::anyhow!(
            "Post number {} of topic with ID {} could not be found",
            post_number,
            topic_id
        ))
    }

    async fn update_post_content(&self, post_id: u64, new_content: String) -> anyhow::Result<()> {
        let payload = json!({
            "post": {
                "raw": new_content
            }
        });
        let payload = serde_json::to_string(&payload)?;

        self.request::<serde_json::Value>(format!("posts/{}.json", post_id), Method::PUT, Some(payload), None)
            .await?;
        Ok(())
    }

    fn format_url_for_automatic_topic_creation(&self, topic: DiscourseTopic) -> anyhow::Result<Url> {
        let url: Url = self.forum_url.parse()?;
        let mut url = url.join("new-topic")?;
        url.query_pairs_mut()
            .append_pair("title", &topic.title)
            .append_pair("body", &topic.content)
            .append_pair("category", &topic.category)
            .append_pair("tags", &topic.tags.join(","));
        Ok(url)
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

const SUBNET_TOPICS_AND_SLUGS: &str = include_str!("../assets/subnet_topic_map.json");
fn get_subnet_topics_map() -> BTreeMap<PrincipalId, SubnetTopicInfo> {
    serde_json::from_str(SUBNET_TOPICS_AND_SLUGS).unwrap()
}

fn get_subnet_topics_from_path(path: &Path) -> anyhow::Result<BTreeMap<PrincipalId, SubnetTopicInfo>> {
    let file = std::fs::File::open(path)?;
    serde_json::from_reader(file).map_err(anyhow::Error::from)
}

#[derive(Debug)]
pub struct DiscourseResponse {
    pub url: String,
    pub update_id: u64,
    pub is_topic: bool,
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
    fn generate_link_topic_creation() {
        let discourse_client = DiscourseClientImp::new("https://forum.dfinity.org".to_string(), "".to_string(), "".to_string()).unwrap();

        let link = discourse_client
            .format_url_for_automatic_topic_creation(DiscourseTopic {
                category: NNS_PROPOSAL_DISCUSSION.to_string(),
                content: "Test content".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                title: "Test automatic forum post creation".to_string(),
            })
            .unwrap();

        assert_eq!(link.to_string(), "https://forum.dfinity.org/new-topic?title=Test+automatic+forum+post+creation&body=Test+content&category=NNS+proposal+discussions&tags=tag1%2Ctag2")
    }
}
