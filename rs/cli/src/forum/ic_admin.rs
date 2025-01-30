use std::sync::Arc;

use log::info;

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
        ic_admin,
        simulate: ctx.is_dry_run() || ctx.is_offline(),
        forum_parameters: forum_parameters.clone(),
    }
}

impl IcAdminProxy {
    async fn propose_or_submit(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind, directly_submit: bool) -> anyhow::Result<()> {
        let forum_post = super::ForumContext::from_opts(&self.forum_parameters, self.simulate)
            .client()?
            .forum_post(kind)
            .await?;
        let opts = ProposeOptions {
            forum_post_link: forum_post.url().map(|s| s.into()),
            ..opts
        };
        let res = if directly_submit {
            self.ic_admin.propose_submit(cmd, opts).await?
        } else {
            self.ic_admin.propose_run(cmd, opts).await?
        };
        if self.simulate {
            info!("Simulating that the proposal returned by ic-admin is 123456");
            forum_post.add_proposal_url(123456).await
        } else {
            forum_post.update_by_parsing_ic_admin_response(res).await
        }
    }
    pub async fn propose_run(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind) -> anyhow::Result<()> {
        self.propose_or_submit(cmd, opts, kind, false).await
    }

    pub async fn propose_submit(&self, cmd: ProposeCommand, opts: ProposeOptions, kind: ForumPostKind) -> anyhow::Result<()> {
        self.propose_or_submit(cmd, opts, kind, true).await
    }
}
