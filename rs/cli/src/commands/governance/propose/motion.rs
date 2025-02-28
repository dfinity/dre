use std::path::PathBuf;

use clap::Args;
use ic_nns_governance_api::pb::v1::{MakeProposalRequest, Motion as MotionPayload, ProposalActionRequest};
use tokio::io::AsyncReadExt;

use crate::{
    auth::AuthRequirement,
    exe::args::GlobalArgs,
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
    util::{extract_title_and_text, utf8},
};

#[derive(Args, Debug, Clone)]
#[group(multiple = true)]
/// Motion parameters.
struct MotionParameters {
    /// File containing summary of the proposal, customarily written in Markdown format, max 30 KiB; if "-", read the summary from standard input;
    /// if no explicit --title is specified, the first line found in the text will be stripped from the summary if it is a Markdown
    /// level-1 heading
    #[arg(num_args(1), help_heading = "Motion parameters")]
    pub summary_file: PathBuf,

    /// Title to give to the proposal; defaults to the first Markdown level-1 heading of the summary, which will be stripped from
    /// the summary in the default case.  For this default to kick in, the heading must be at the top of the summary
    #[arg(long, help_heading = "Motion parameters")]
    pub title: Option<String>,

    /// File containing text for the motion text field, customarily written in Markdown format, max 100 KiB; if no option is specified, a brief
    /// blurb asking the reader to refer to the summary is placed instead
    #[arg(long, help_heading = "Motion parameters", conflicts_with = "motion_text")]
    pub motion_text_file: Option<PathBuf>,

    /// Text for the motion text field, customarily written in Markdown format, max 100 KiB; if no option is specified, a brief
    /// blurb asking the reader to refer to the summary is placed instead
    #[arg(long, help_heading = "Motion parameters", conflicts_with = "motion_text_file")]
    pub motion_text: Option<String>,
}

#[derive(Args, Debug)]
/// Submit a new motion.
pub struct Motion {
    #[command(flatten)]
    parameters: MotionParameters,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Motion {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let summary = match self.parameters.summary_file.as_path().as_os_str().to_str() {
            Some("-") => {
                let mut ret: String = "".to_string();
                tokio::io::stdin().read_to_string(&mut ret).await?;
                ret
            }
            _ => utf8(tokio::fs::read(&self.parameters.summary_file).await?, "Summary must be valid UTF-8")?,
        };
        let (title, summary) = match &self.parameters.title {
            Some(s) => (Some(s.clone()), summary.clone()),
            None => extract_title_and_text(&summary),
        };
        let motion_text = match (&self.parameters.motion_text, &self.parameters.motion_text_file) {
            (Some(motion_text), _) => motion_text.clone(),
            (_, Some(motion_text_file)) => utf8(tokio::fs::read(&motion_text_file).await?, "Motion text must be valid UTF-8")?,
            (None, None) => "Please refer to the summary of the proposal for the contents of the motion.".to_string(),
        };

        let request = MakeProposalRequest {
            title: title.clone(),
            summary: summary.clone(),
            url: "<to be supplied or generated later>".into(),
            action: Some(ProposalActionRequest::Motion(MotionPayload { motion_text })),
        };

        Submitter::from(&self.submission_parameters)
            .propose_and_print(
                ctx.governance_executor().await?.execution(request),
                ForumPostKind::Motion {
                    title: title.clone(),
                    summary: summary.clone(),
                },
            )
            .await
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {
        let _ = _args;
    }
}
