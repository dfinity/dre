use std::sync::Arc;

use log::{info, warn};

use super::{ForumParameters, ForumPostKind};
use crate::{
    ctx::DreContext,
    ic_admin::{IcAdmin, ProposeCommand, ProposeOptions},
};

pub struct IcAdminProxy {
    ic_admin: Arc<dyn IcAdmin>,
    simulate: bool,
    forum_parameters: ForumParameters,
}

pub fn forum_enabled_proposer(forum_parameters: &ForumParameters, ctx: &DreContext, ic_admin: Arc<dyn IcAdmin>) -> IcAdminProxy {
    IcAdminProxy {
        ic_admin: ic_admin.clone(),
        simulate: ctx.is_dry_run() || ctx.is_offline(),
        forum_parameters: forum_parameters.clone(),
    }
}

impl IcAdminProxy {
    async fn propose(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind) -> anyhow::Result<()> {
        if !self
            .ic_admin
            .propose_print_and_confirm(
                cmd.clone(),
                ProposeOptions {
                    forum_post_link: Some("<forum post URL will be supplied once you confirm>".into()),
                    ..opts.clone()
                },
            )
            .await?
        {
            return Ok(());
        };

        let forum_post = super::ForumContext::from_opts(&self.forum_parameters, self.simulate)
            .client()?
            .forum_post(kind)
            .await?;
        match self
            .ic_admin
            .propose_submit(
                cmd,
                ProposeOptions {
                    forum_post_link: forum_post.url().map(|s| s.into()),
                    ..opts
                },
            )
            .await
        {
            Ok(res) => {
                if self.simulate {
                    info!("Simulating that the proposal returned by ic-admin is 123456");
                    forum_post.add_proposal_url(123456).await
                } else {
                    forum_post.update_by_parsing_ic_admin_response(res).await
                }
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
    pub async fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind) -> anyhow::Result<()> {
        self.propose(cmd, opts, kind).await
    }

    pub async fn propose_submit(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind) -> anyhow::Result<()> {
        self.propose(cmd, opts, kind).await
    }
}
