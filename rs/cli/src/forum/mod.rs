use std::{path::PathBuf, str::FromStr};

use clap::Args as ClapArgs;
use futures::future::BoxFuture;
use ic_types::PrincipalId;
use log::warn;
use mockall::automock;

use crate::{proposal_executors::ProposalExecution, util::yesno};

mod impls;

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
    // FIXME can we hide this structure altogether?
    #[clap(long, env = "FORUM_POST_LINK", help_heading = "Proposal URL parameters", visible_aliases = &["forum-link", "forum", "proposal-url"], default_value = "ask", value_parser = clap::value_parser!(ForumPostLinkVariant), help = r#"Forum link post handling method. Options:
* The word 'discourse' to ask the embedded Discourse client to auto create a post or a topic, and update the forum post after proposal submission.
    See Discourse forum interaction parameters for information on how to authenticate.
* A plain URL or the word 'ask' to prompt you for a link.
    Note that the IC will reject links not under forum.dfinity.org, and you are on the hook for updating the URL to reflect any proposal submission.
* The word 'omit' to omit the link.
    While you can submit proposals without a link, this is highly discouraged.
"#)]
    pub(crate) forum_post_link: ForumPostLinkVariant,

    /// Api key used to interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_KEY",
        help_heading = "Discourse forum interaction parameters",
        hide_env_values = true
    )]
    discourse_api_key: Option<String>,

    /// Api user that will interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_USER",
        help_heading = "Discourse forum interaction parameters",
        default_value = "DRE-Team"
    )]
    discourse_api_user: Option<String>,

    /// Api url used to interact with the forum
    #[clap(
        long,
        env = "DISCOURSE_API_URL",
        help_heading = "Discourse forum interaction parameters",
        default_value = "https://forum.dfinity.org"
    )]
    discourse_api_url: String,

    /// Skip forum post creation all together, also will not
    /// prompt user for the link
    #[clap(long, env = "DISCOURSE_SKIP_POST_CREATION", help_heading = "Discourse forum interaction parameters")]
    discourse_skip_post_creation: bool,

    #[clap(
        long,
        env = "DISCOURSE_SUBNET_TOPIC_OVERRIDE_FILE_PATH",
        help_heading = "Discourse forum interaction parameters"
    )]
    discourse_subnet_topic_override_file_path: Option<PathBuf>,
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

    pub fn forum_post_link_mandatory(&self) -> anyhow::Result<()> {
        if let ForumPostLinkVariant::Omit = self.forum_post_link {
            return Err(anyhow::anyhow!("Forum post link cannot be omitted for this subcommand.",));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HowToProceed {
    Confirm,
    Unconditional,
    DryRun,
    #[allow(dead_code)]
    UnitTests, // Necessary for unit tests, otherwise confirmation is requested.
               // Generally this is hit when DreContext (created by get_mocked_ctx) has
               // both dry_run and proceed_without_confirmation set to true.
               // The net effect is that both the dry run and the final command are run.
               // FIXME we should probably rename this to "DuringTesting".
}

#[derive(ClapArgs, Debug, Clone)]

/// Options for commands that may require confirmation.
pub struct ConfirmationModeOptions {
    /// To skip the confirmation prompt
    #[clap(
        short,
        long,
        global = true,
        env = "YES",
        conflicts_with = "dry_run",
        help_heading = "Options on how to proceed",
        help = "Do not ask for confirmation. If specified, the operation will be performed without requesting any confirmation from you."
    )]
    yes: bool,

    #[clap(long, aliases = [ "dry-run", "dryrun", "simulate", "no"], env = "DRY_RUN", global = true, conflicts_with = "yes", help = r#"Dry-run, or simulate operation. If specified will not make any changes; instead, it will show what would be done or submitted."#,help_heading = "Options on how to proceed")]
    dry_run: bool,
}

impl ConfirmationModeOptions {
    /// Return an option set for unit tests, not instantiable via command line due to conflict.
    pub fn for_unit_tests() -> Self {
        ConfirmationModeOptions { yes: true, dry_run: true }
    }
}

impl From<&ConfirmationModeOptions> for HowToProceed {
    fn from(o: &ConfirmationModeOptions) -> Self {
        match (o.dry_run, o.yes) {
            (false, true) => Self::Unconditional,
            (true, false) => Self::DryRun,
            (false, false) => Self::Confirm,
            (true, true) => Self::UnitTests, // This variant cannot be instantiated via the command line.
        }
    }
}

#[derive(ClapArgs, Debug, Clone)]
pub struct SubmissionParameters {
    #[clap(flatten)]
    pub forum_parameters: ForumParameters,

    #[clap(flatten)]
    pub confirmation_mode: ConfirmationModeOptions,
}

// FIXME: this should become part of a new composite trait
// that builds on the ProducesProposalResults trait,
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
struct ForumContext {
    forum_opts: ForumParameters,
}

#[allow(clippy::too_many_arguments)]
impl ForumContext {
    fn new(forum_opts: ForumParameters) -> Self {
        Self { forum_opts }
    }

    // FIXME: turn into impl From.
    fn from_opts(opts: &ForumParameters) -> Self {
        Self::new(opts.clone())
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

/// Helps the caller preview and then submit a proposal automatically,
/// handling the forum post part of the work as smoothly as possible.
pub struct Submitter {
    mode: HowToProceed,
    forum_parameters: ForumParameters,
}

impl From<&SubmissionParameters> for Submitter {
    fn from(other: &SubmissionParameters) -> Self {
        Self {
            mode: (&other.confirmation_mode).into(),
            forum_parameters: other.forum_parameters.clone(),
        }
    }
}

impl Submitter {
    /// Submits a proposal (maybe in dry-run mode) with confirmation from the user, unless the user
    /// specifies in the command line that he wants no confirmation (--yes).
    pub async fn propose(&self, execution: Box<dyn ProposalExecution>, kind: ForumPostKind) -> anyhow::Result<()> {
        if let HowToProceed::Unconditional = self.mode {
        } else {
            execution.simulate().await?;
        };

        if let HowToProceed::Confirm = self.mode {
            // Ask for confirmation
            if !yesno("Do you want to continue?", false).await?? {
                return Ok(());
            }
        }

        if let HowToProceed::DryRun = self.mode {
            Ok(())
        } else {
            let forum_post = ForumContext::from_opts(&self.forum_parameters).client()?.forum_post(kind).await?;
            let res = execution.submit(forum_post.url()).await;
            match res {
                Ok(res) => forum_post.add_proposal_url(res.into()).await,
                Err(e) => {
                    if let Some(forum_post_url) = forum_post.url() {
                        // Here we would ask the forum post code to delete the post since
                        // the submission has failed... that is, if we had that feature.
                        warn!(
                        "Forum post {} may have been created for this proposal, but proposal submission failed.  Please delete the forum post if necessary, as it now serves no purpose.",
                        forum_post_url
                    );
                    };
                    Err(e)
                }
            }
        }
    }
}
