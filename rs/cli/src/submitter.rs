use clap::Args as ClapArgs;
use log::warn;

use crate::{
    confirm::{ConfirmationModeOptions, HowToProceed},
    forum::{ForumContext, ForumParameters, ForumPostKind},
    proposal_executors::ProposalExecution,
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
            let forum_post = ForumContext::from(&self.forum_parameters).client()?.forum_post(kind).await?;
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
