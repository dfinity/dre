use std::path::PathBuf;

use clap::Args;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_governance_api::pb::v1::{MakeProposalRequest, Motion as MotionPayload, ProposalActionRequest};
use tokio::io::AsyncReadExt;

use crate::commands::{AuthRequirement, ExecutableCommand};
use ic_canisters::governance::GovernanceCanisterWrapper;

#[derive(Args, Debug, Clone)]
#[group(multiple = true)]
/// Motion parameters.
struct MotionParameters {
    /// File containing summary of the proposal, customarily written in Markdown format, max 30 KiB; if "-", read the summary from standard input;
    /// if no explicit --title is specified, the first line found in the text will be stripped from the summary if it is a Markdown
    /// level-1 heading
    #[arg(num_args(1), help_heading = "Command parameters")]
    pub summary_file: PathBuf,

    /// Title to give to the proposal; defaults to the first Markdown level-1 heading of the summary, which will be stripped from
    /// the summary in the default case.  For this default to kick in, the heading must be at the top of the summary
    #[arg(long, help_heading = "Command parameters")]
    pub title: Option<String>,

    /// File containing text for the motion text field, customarily written in Markdown format, max 100 KiB; if no option is specified, a brief
    /// blurb asking the reader to refer to the summary is placed instead
    #[arg(long, help_heading = "Command parameters", conflicts_with = "motion_text")]
    pub motion_text_file: Option<PathBuf>,

    /// Text for the motion text field, customarily written in Markdown format, max 100 KiB; if no option is specified, a brief
    /// blurb asking the reader to refer to the summary is placed instead
    #[arg(long, help_heading = "Command parameters", conflicts_with = "motion_text_file")]
    pub motion_text: Option<String>,

    /// URL typically used to discuss the proposal; proposals risk being rejected without a valid venue to discuss them
    #[arg(long, help_heading = "Command parameters")]
    pub proposal_url: url::Url,
}

#[derive(Args, Debug)]
/// Submit a new motion.
pub struct Motion {
    #[command(flatten)]
    parameters: MotionParameters,
}

impl Motion {
    fn extract_title_and_text(&self, raw_text: &str) -> (Option<String>, String) {
        // Step 1: Remove leading/trailing empty lines (including lines with only whitespace).
        let lines: Vec<&str> = raw_text.lines().skip_while(|line| line.trim().is_empty()).collect();
        let lines: Vec<&str> = lines.into_iter().rev().skip_while(|line| line.trim().is_empty()).collect();
        let lines: Vec<&str> = lines.into_iter().rev().collect();

        // Step 2: Parse the first line as a title if it starts with '# '
        if lines.is_empty() {
            // If no lines remain after trimming, there's no title or body
            return (None, "".to_string());
        }

        let first_line = lines[0];
        // CommonMark-compliant H1 headings MUST start with "# ", so starts_with("# ") would be enough.
        // To tolerate other “flavors” of Markdown that allow #Title without a space, we use a more complex check.
        let (title, body) = if first_line.starts_with('#') && !first_line.starts_with("##") {
            // Strip out '#' plus any extra leading space
            let stripped_title = first_line.trim_start_matches('#').trim_start();
            let body = lines[1..].join("\n"); // Join the remaining lines
            (Some(stripped_title.to_owned()), body.trim_start().to_string())
        } else {
            (None, lines.join("\n"))
        };
        (title, body)
    }
}

impl ExecutableCommand for Motion {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (neuron, client) = ctx.create_ic_agent_canister_client().await?;
        let governance = GovernanceCanisterWrapper::from(client);
        let summary = match self.parameters.summary_file.as_path().as_os_str().to_str() {
            Some("-") => {
                let mut ret: String = "".to_string();
                tokio::io::stdin().read_to_string(&mut ret).await?;
                ret
            }
            _ => {
                let res = tokio::fs::read(&self.parameters.summary_file).await?;
                String::from_utf8(res).map_err(|e| anyhow::anyhow!("Summary must be valid UTF-8: {}", e))?
            }
        };
        let (title, summary) = match &self.parameters.title {
            Some(s) => (Some(s.clone()), summary.clone()),
            None => self.extract_title_and_text(&summary),
        };
        let motion_text = match (&self.parameters.motion_text, &self.parameters.motion_text_file) {
            (Some(motion_text), _) => motion_text.clone(),
            (_, Some(motion_text_file)) => {
                let res = tokio::fs::read(&motion_text_file).await?;
                String::from_utf8(res).map_err(|e| anyhow::anyhow!("Summary must be valid UTF-8: {}", e))?
            }
            (None, None) => "Please refer to the summary of the proposal for the contents of the motion.".to_string(),
        };
        let proposal = MakeProposalRequest {
            title,
            summary,
            url: self.parameters.proposal_url.to_string(),
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
