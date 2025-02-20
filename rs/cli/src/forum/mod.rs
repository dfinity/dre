use std::{path::PathBuf, str::FromStr};

use clap::Args as ClapArgs;
use futures::future::BoxFuture;
use ic_types::PrincipalId;
use mockall::automock;

mod impls;

#[derive(Debug, Clone)]
pub(crate) enum ForumPostLinkVariant {
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
    #[clap(long, global=true, env = "FORUM_POST_LINK", help_heading = "Proposal URL parameters", visible_aliases = &["forum-link", "forum", "proposal-url"], default_value = "ask", value_parser = clap::value_parser!(ForumPostLinkVariant), help = r#"Forum link post handling method. Options:
* The word 'discourse' to ask the embedded Discourse client to auto create a post or a topic, and update the forum post after proposal submission.
    See Discourse forum interaction parameters for information on how to authenticate.
* A plain URL or the word 'ask' to prompt you for a link.
    Note that the IC will reject links not under forum.dfinity.org, and you are on the hook for updating the URL to reflect any proposal submission.
* The word 'omit' to omit the link.
    While you can submit proposals without a link, this is highly discouraged.
"#)]
    forum_post_link: ForumPostLinkVariant,

    /// Api key used to interact with the forum
    #[clap(
        long,
        global = true,
        env = "DISCOURSE_API_KEY",
        help_heading = "Discourse forum interaction parameters",
        hide_env_values = true
    )]
    discourse_api_key: Option<String>,

    /// Api user that will interact with the forum
    #[clap(
        long,
        global = true,
        env = "DISCOURSE_API_USER",
        help_heading = "Discourse forum interaction parameters",
        default_value = "DRE-Team"
    )]
    discourse_api_user: Option<String>,

    /// Api url used to interact with the forum
    #[clap(
        long,
        global = true,
        env = "DISCOURSE_API_URL",
        help_heading = "Discourse forum interaction parameters",
        default_value = "https://forum.dfinity.org"
    )]
    discourse_api_url: String,

    /// Skip forum post creation all together, also will not
    /// prompt user for the link
    #[clap(
        long,
        global = true,
        env = "DISCOURSE_SKIP_POST_CREATION",
        help_heading = "Discourse forum interaction parameters"
    )]
    discourse_skip_post_creation: bool,

    #[clap(
        long,
        global = true,
        env = "DISCOURSE_SUBNET_TOPIC_OVERRIDE_FILE_PATH",
        help_heading = "Discourse forum interaction parameters"
    )]
    discourse_subnet_topic_override_file_path: Option<PathBuf>,
}

impl ForumParameters {
    #[cfg(test)]
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

    #[cfg(test)]
    pub fn with_post_link(self, url: url::Url) -> Self {
        Self {
            forum_post_link: ForumPostLinkVariant::Url(url),
            ..self
        }
    }

    pub fn forum_post_link_mandatory(&self) -> anyhow::Result<()> {
        if let ForumPostLinkVariant::Omit = self.forum_post_link {
            return Err(anyhow::anyhow!("Forum post link cannot be omitted for this subcommand.",));
        }
        Ok(())
    }
}

// FIXME: this should become part of a new composite trait
// that builds on the Proposable trait,
// so that we don't have to have a separate kind here, this just
// becomes a trait or an impl, and the intelligence needed to compose
// the forum post can be decentralized to the right places in the code,
// instead of living divorced from the proposal type itself.
pub enum ForumPostKind {
    ReplaceNodes { subnet_id: PrincipalId, body: String },
    AuthorizedSubnetsUpdate { body: String },
    Motion { title: Option<String>, summary: String },
    Generic,
}

#[automock]
pub trait ForumPostHandler: Sync + Send {
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

#[derive(Clone)]
pub struct ForumContext {
    forum_opts: ForumParameters,
}

#[allow(clippy::too_many_arguments)]
impl ForumContext {
    fn new(forum_opts: ForumParameters) -> Self {
        Self { forum_opts }
    }

    pub fn client(&self) -> anyhow::Result<Box<dyn ForumPostHandler>> {
        match &self.forum_opts.forum_post_link {
            ForumPostLinkVariant::Url(u) => Ok(Box::new(impls::UserSuppliedLink { url: Some(u.clone()) }) as Box<dyn ForumPostHandler>),
            ForumPostLinkVariant::Omit => Ok(Box::new(impls::UserSuppliedLink { url: None }) as Box<dyn ForumPostHandler>),
            ForumPostLinkVariant::Ask => Ok(Box::new(impls::Prompter {}) as Box<dyn ForumPostHandler>),
            ForumPostLinkVariant::ManageOnDiscourse => Ok(Box::new(impls::Discourse::new(self.forum_opts.clone())?) as Box<dyn ForumPostHandler>),
        }
    }
}

impl From<&ForumParameters> for ForumContext {
    fn from(p: &ForumParameters) -> Self {
        Self::new(p.clone())
    }
}
