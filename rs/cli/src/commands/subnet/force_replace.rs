use std::collections::BTreeSet;

use clap::Args;
use ic_types::PrincipalId;
use itertools::Itertools;

use crate::{exe::ExecutableCommand, submitter::SubmissionParameters};

#[derive(Args, Debug)]
pub struct ForceReplace {
    /// Subnet id to perform force replacement from
    #[clap(long)]
    subnet_id: PrincipalId,

    /// Nodes to remove from the given subnet
    #[clap(long, num_args = 1..)]
    from: Vec<PrincipalId>,

    /// Nodes to include into a given subnet
    #[clap(long, num_args = 1..)]
    to: Vec<PrincipalId>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for ForceReplace {
    fn require_auth(&self) -> crate::auth::AuthRequirement {
        crate::auth::AuthRequirement::Neuron
    }

    fn validate(&self, _args: &crate::exe::args::GlobalArgs, cmd: &mut clap::Command) {
        let from: BTreeSet<PrincipalId> = self.from.iter().cloned().collect();
        let to: BTreeSet<PrincipalId> = self.to.iter().cloned().collect();

        if from.len() != to.len() {
            cmd.error(
                clap::error::ErrorKind::InvalidValue,
                format!("`from` and `to` have to contain the same number of elements"),
            )
            .exit();
        }

        let duplicates = from.intersection(&to).collect_vec();

        if duplicates.is_empty() {
            return;
        }

        let duplicates = duplicates.iter().map(|p| p.to_string().split_once("-").unwrap().0.to_string()).join(", ");

        cmd.error(
            clap::error::ErrorKind::ValueValidation,
            format!("`from` and `to` contain the following duplicates: [{duplicates}]"),
        )
        .exit()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
