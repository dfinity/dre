use clap::Args as ClapArgs;
use log::warn;

use crate::{
    confirm::{ConfirmationModeOptions, HowToProceed},
    forum::{ForumContext, ForumParameters, ForumPostKind},
    proposal_executors::{ProposalExecution, ProposalResponseWithId},
    util::yesno,
};

#[derive(ClapArgs, Debug, Clone)]
pub struct SubmissionParameters {
    #[clap(flatten)]
    pub forum_parameters: ForumParameters,

    #[clap(flatten)]
    pub confirmation_mode: ConfirmationModeOptions,
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
    ///
    /// When a proposal has successfully been executed (as opposed to just simulated),
    /// the result will contain a Some(ProposalResponseWithId).
    #[must_use = "Are you implementing a CLI command?  Did you forget to print the proposal ID possibly returned by this function?  Consider propose_and_print instead."]
    pub async fn propose(&self, execution: Box<dyn ProposalExecution>, kind: ForumPostKind) -> anyhow::Result<Option<ProposalResponseWithId>> {
        if let HowToProceed::Unconditional = self.mode {
        } else {
            execution.simulate(self.forum_parameters.forum_post_link_for_simulation()).await?;
        };

        if let HowToProceed::Confirm = self.mode {
            // Ask for confirmation
            if !yesno("Do you want to continue?", false).await?? {
                return Ok(None);
            }
        }

        if let HowToProceed::DryRun = self.mode {
            Ok(None)
        } else {
            let forum_post = ForumContext::from(&self.forum_parameters).client()?.forum_post(kind).await?;
            let res = execution.submit(forum_post.url()).await;
            match res {
                Ok(res) => {
                    match forum_post.add_proposal_url(res.clone().into()).await {
                        Ok(_) => (),
                        Err(e) => {
                            if let Some(forum_post_url) = forum_post.url() {
                                warn!("Failed to add the proposal URL to forum post {}: {}", forum_post_url, e)
                            } else {
                                warn!("Failed to add the proposal URL to forum post: {}", e)
                            }
                        }
                    };
                    Ok(Some(res))
                }
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

    /// Submits a proposal (maybe in dry-run mode) with confirmation from the user, by calling
    /// Self.propose().
    ///
    /// When a proposal has successfully been executed (as opposed to just simulated),
    /// the proposal ID will be printed to standard output as "proposal XXXXXX".
    ///
    /// You should only call this convenience method if all your code is going to do is print
    /// the returned proposal ID and exit.
    pub async fn propose_and_print(&self, execution: Box<dyn ProposalExecution>, kind: ForumPostKind) -> anyhow::Result<()> {
        if let Some(p) = self.propose(execution, kind).await? {
            println!("{}", p)
        };
        Ok(())
    }
}
