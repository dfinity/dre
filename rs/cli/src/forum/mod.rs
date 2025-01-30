use std::{path::PathBuf, str::FromStr};

use clap::Args as ClapArgs;
use futures::future::BoxFuture;
use ic_types::PrincipalId;
use mockall::automock;
use regex::Regex;

use crate::ctx::DreContext;

pub mod ic_admin;
pub mod impls;

#[derive(Debug, Clone)]
pub enum ForumPostLinkVariant {
    Url(url::Url),
    ManageOnDiscourse,
    Ask,
    Omit,
}

impl FromStr for ForumPostLinkVariant {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "discourse" => Ok(Self::ManageOnDiscourse),
            "omit" => Ok(Self::Omit),
            "ask" => Ok(Self::Ask),
            _ => match url::Url::from_str(s) {
                Ok(u) => Ok(Self::Url(u)),
                Err(e) => Err(format!("Invalid forum post link {}: {}", s, e)),
            },
        }
    }
}

#[derive(ClapArgs, Debug, Clone)]
pub struct ForumParameters {
    /// Link to the related forum post, where proposal details can be discussed, or "discourse", or "ask"; the magic value "discourse" causes a forum post to be created on Discourse automatically in some cases, or may ask you to create a post yourself on Discourse, and will update the post after the proposal, but it requires the Discourse API key and user parameters to be specified; the magic value "ask" (the default) causes DRE to interactively prompt you for a forum post link, which will not be updated; the magic value "omit" causes DRE to omit any forum post link
    #[clap(long, env = "FORUM_POST_LINK", help_heading = "Proposal URL parameters", visible_aliases = &["forum-link", "forum", "proposal-url"], default_value = "ask", value_parser = clap::value_parser!(ForumPostLinkVariant))]
    pub forum_post_link: ForumPostLinkVariant,

    /// Api key used to interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_KEY",
        help_heading = "Discourse forum interaction parameters",
        hide_env_values = true
    )]
    pub(crate) discourse_api_key: Option<String>,

    /// Api user that will interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_USER",
        help_heading = "Discourse forum interaction parameters",
        default_value = "DRE-Team"
    )]
    pub(crate) discourse_api_user: Option<String>,

    /// Api url used to interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_URL",
        help_heading = "Discourse forum interaction parameters",
        default_value = "https://forum.dfinity.org"
    )]
    pub(crate) discourse_api_url: String,

    /// Skip forum post creation all together, also will not
    /// prompt user for the link
    #[clap(long, env = "DISCOURSE_SKIP_POST_CREATION", help_heading = "Discourse forum interaction parameters")]
    pub(crate) discourse_skip_post_creation: bool,

    #[clap(
        long,
        env = "DISCOURSE_SUBNET_TOPIC_OVERRIDE_FILE_PATH",
        help_heading = "Discourse forum interaction parameters"
    )]
    pub(crate) discourse_subnet_topic_override_file_path: Option<PathBuf>,
}

impl ForumParameters {
    pub fn disable_forum() -> Self {
        Self {
            forum_post_link: ForumPostLinkVariant::Omit,
            discourse_api_key: None,
            discourse_api_user: None,
            discourse_api_url: "http://localhost/".to_string(),
            discourse_skip_post_creation: true,
            discourse_subnet_topic_override_file_path: None,
        }
    }
}

pub enum ForumPostKind {
    ReplaceNodes { subnet_id: PrincipalId, body: String },
    AuthorizedSubnetsUpdate { body: String },
    Motion { title: Option<String>, summary: String },
    Generic,
}

#[automock]
pub trait ForumClient: Sync + Send {
    #[must_use = "You must not forget to update the proposal URL using the forum post this returns"]
    fn forum_post(&self, kind: ForumPostKind) -> BoxFuture<'_, anyhow::Result<Box<dyn ForumPost>>>;
}

#[automock]
pub trait ForumPost: Sync + Send {
    #[must_use = "You must not forget to use the forum post URL in the proposal you are about to make"]
    fn url(&self) -> Option<url::Url>;

    #[must_use = "You must not forget to update the proposal URL in the forum post you requested"]
    fn add_proposal_url(&self, proposal_id: u64) -> BoxFuture<'_, anyhow::Result<()>>;
}

impl dyn ForumPost {
    pub async fn update_by_parsing_ic_admin_response(&self, ic_admin_response: String) -> anyhow::Result<()> {
        let proposal_id = parse_proposal_id_from_ic_admin_response(ic_admin_response)?;
        self.add_proposal_url(proposal_id).await
    }
}

pub fn client(forum_parameters: &ForumParameters, ctx: &DreContext) -> anyhow::Result<Box<dyn ForumClient>> {
    ForumContext::from_opts(forum_parameters, ctx.is_dry_run() || ctx.is_offline()).client()
}

#[derive(Clone)]
struct ForumContext {
    simulate: bool,
    forum_opts: ForumParameters,
}

#[allow(clippy::too_many_arguments)]
impl ForumContext {
    fn new(simulate: bool, forum_opts: ForumParameters) -> Self {
        Self { simulate, forum_opts }
    }

    fn from_opts(opts: &ForumParameters, simulate: bool) -> Self {
        Self::new(simulate, opts.clone())
    }

    pub fn client(&self) -> anyhow::Result<Box<dyn ForumClient>> {
        match &self.forum_opts.forum_post_link {
            ForumPostLinkVariant::Url(u) => Ok(Box::new(impls::OptionalLinkClient { url: Some(u.clone()) }) as Box<dyn ForumClient>),
            ForumPostLinkVariant::Omit => Ok(Box::new(impls::OptionalLinkClient { url: None }) as Box<dyn ForumClient>),
            ForumPostLinkVariant::Ask => Ok(Box::new(impls::PromptClient {}) as Box<dyn ForumClient>),
            ForumPostLinkVariant::ManageOnDiscourse => {
                Ok(Box::new(impls::DiscourseClient::new(self.forum_opts.clone(), self.simulate)?) as Box<dyn ForumClient>)
            }
        }
    }
}

fn parse_proposal_id_from_ic_admin_response(response: String) -> anyhow::Result<u64> {
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
