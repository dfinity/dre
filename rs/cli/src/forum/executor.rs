use std::{fmt::Display, str::FromStr};

use log::{info, warn};

use super::{ForumParameters, ForumPostKind};
use crate::{
    ctx::HowToProceed,
    ic_admin::{ProposableViaIcAdmin, ProposalExecutor, ProposalId},
    util::yesno,
};

pub struct ForumEnabledProposalExecutor {
    executor: ProposalExecutor,
    mode: HowToProceed,
    forum_parameters: ForumParameters,
}

impl ForumEnabledProposalExecutor {
    pub fn from_executor_and_mode(forum_parameters: &ForumParameters, mode: HowToProceed, executor: ProposalExecutor) -> Self {
        Self {
            executor,
            mode,
            forum_parameters: forum_parameters.clone(),
        }
    }

    /// Submits a proposal (maybe in dry-run mode) with confirmation from the user, unless the user
    /// specifies in the command line that he wants no confirmation (--yes).
    pub async fn propose<T>(&self, exe: T, kind: ForumPostKind) -> anyhow::Result<()>
    where
        T: ProposableViaIcAdmin,
        T::Output: ProposalId,
        <<T as ProposableViaIcAdmin>::Output as FromStr>::Err: Display,
    {
        let executor = &self.executor;

        if let HowToProceed::Unconditional = self.mode {
        } else {
            executor.simulate(&exe).await?;
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
            let forum_post = super::ForumContext::from_opts(&self.forum_parameters).client()?.forum_post(kind).await?;
            let res = executor.submit(&exe, forum_post.url()).await;
            match res {
                Ok(res) => {
                    info!("Submitted proposal has ID {}", res.id());
                    forum_post.add_proposal_url(res.id()).await
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
}
