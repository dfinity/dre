use std::path::PathBuf;

use clap::Args;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_governance_api::pb::v1::{MakeProposalRequest, Motion as MotionPayload, ProposalActionRequest};
use tokio::io::AsyncReadExt;

use crate::commands::{AuthRequirement, ExecutableCommand};
use ic_canisters::governance::GovernanceCanisterWrapper;

#[derive(Args, Debug)]
/// Submit a new motion.
pub struct Motion {
    /// File containing text of the proposal, customarily written in Markdown format; if "-", read the text from standard input.
    /// If no explicit --title is specified, the first headline found in the text will be stripped from the motion text.
    #[clap(num_args(1..))]
    pub motion_text_file: PathBuf,

    /// Title to give to the proposal; defaults to the first Markdown level-1 heading of the motion text, which will be stripped from
    /// the summary in the default case.  For this default to kick in, the heading must be at the top of the motion text.
    #[clap(long)]
    pub title: Option<String>,

    // ATTENTION REVIEWERS: should we put the entire text of the proposal in the summary?  Or should we intelligently extract the
    // first paragraph and use that?
    /// Summary to give to the proposal; defaults to the text of the motion.
    #[clap(long)]
    pub summary: Option<String>,

    /// URL for discussion of the proposal; defaults to no the DFINITY forum governance topic.
    #[clap(long, default_value = "https://forum.dfinity.org/c/governance/27")]
    pub url: url::Url,
}

impl Motion {
    fn extract_title_and_text(&self, text: &String) -> (Option<String>, String) {
        let mytext = text.trim_start();
        let (title, text) = if mytext.starts_with("#") && (mytext.starts_with("# ") || !mytext.starts_with("##")) {
            let mut it = text.lines();
            let title = it.next();
            let text = it.collect::<Vec<_>>().join("\n").trim_start().into();
            (title, text)
        } else {
            (None, text.clone())
        };
        let title = title.map(|s| s.into());
        (title, text)
    }
}

impl ExecutableCommand for Motion {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (neuron, client) = ctx.create_ic_agent_canister_client().await?;
        let governance = GovernanceCanisterWrapper::from(client);
        let motion_text = match self.motion_text_file.as_path().as_os_str().to_str() {
            Some("-") => {
                let mut ret: String = "".to_string();
                tokio::io::stdin().read_to_string(&mut ret).await?;
                ret
            }
            _ => {
                let res = tokio::fs::read(&self.motion_text_file).await?;
                String::from_utf8(res).map_err(|e| anyhow::anyhow!("Motion text must be valid UTF-8: {}", e))?
            }
        };
        let (title, motion_text) = match &self.title {
            Some(s) => (Some(s.clone()), motion_text.clone()),
            None => self.extract_title_and_text(&motion_text),
        };
        let proposal = MakeProposalRequest {
            title,
            summary: match &self.summary {
                Some(s) => s.clone(),
                None => motion_text.clone(),
            },
            url: self.url.to_string(),
            action: Some(ProposalActionRequest::Motion(MotionPayload { motion_text })),
        };
        if ctx.is_dry_run() {
            println!("Proposal that would have been sent:\n{:#?}", proposal);
            return Ok(());
        }
        let propresp = governance.make_proposal(NeuronId { id: neuron.neuron_id }, proposal.into()).await?;
        println!("{:?}", propresp);
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
