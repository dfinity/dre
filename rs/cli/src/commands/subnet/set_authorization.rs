use std::{collections::BTreeSet, sync::Arc};

use clap::Args;
use ic_canisters::cycles_minting::CyclesMintingCanisterWrapper;
use ic_management_types::Subnet;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use indexmap::IndexMap;
use itertools::Itertools;
use log::warn;

use crate::{
    exe::ExecutableCommand,
    forum::ForumPostKind,
    ic_admin::{IcAdminProposal, IcAdminProposalCommand, IcAdminProposalOptions},
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
#[clap(about = "Update subnet authorization")]
pub struct SetAuthorization {
    /// Additional motivation to attach to generated summary.
    #[clap(long)]
    motivation: Option<String>,

    /// The subnets to open.
    #[clap(long)]
    open: Vec<PrincipalId>,

    /// The subnets to close.
    #[clap(long)]
    close: Vec<PrincipalId>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for SetAuthorization {
    fn require_auth(&self) -> crate::auth::AuthRequirement {
        crate::auth::AuthRequirement::Neuron
    }

    fn validate(&self, _args: &crate::exe::args::GlobalArgs, cmd: &mut clap::Command) {
        let mut overlapping_subnets = vec![];

        let open: BTreeSet<PrincipalId> = self.open.iter().cloned().collect();
        let close: BTreeSet<PrincipalId> = self.close.iter().cloned().collect();

        if open.is_empty() && close.is_empty() {
            cmd.error(
                clap::error::ErrorKind::TooFewValues,
                "Both `open` and `closed` arguments were not provided.",
            )
            .exit()
        }

        for subnet in open.intersection(&close) {
            overlapping_subnets.push(subnet.to_string());
        }

        if !overlapping_subnets.is_empty() {
            cmd.error(
                clap::error::ErrorKind::ValueValidation,
                format!("Subnets [{}] found both in `open` and `close` arguments", overlapping_subnets.join(", ")),
            )
            .exit()
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;
        let subnets = registry.subnets().await?;

        let (_, agent) = ctx.create_ic_agent_canister_client().await?;

        let cmc = CyclesMintingCanisterWrapper::from(agent);

        let default_subnets = cmc.get_default_subnets().await?;
        let default_subnets: BTreeSet<PrincipalId> = default_subnets.into_iter().collect();

        let mut new_default_subnets = default_subnets.clone();
        new_default_subnets.extend(self.open.iter());
        new_default_subnets.retain(|s| !self.close.contains(s));

        if default_subnets == new_default_subnets {
            warn!("There are no diffs. Skipping proposal creation.");
            return Ok(());
        }

        let summary = construct_summary(&subnets, &new_default_subnets, default_subnets)?;

        let prop = IcAdminProposal::new(
            IcAdminProposalCommand::SetAuthorizedSubnetworks {
                subnets: new_default_subnets.into_iter().collect(),
            },
            IcAdminProposalOptions {
                title: Some("Updating the list of default subnets".to_string()),
                summary: Some(summary.clone()),
                motivation: self.motivation.clone(),
            },
        );

        Submitter::from(&self.submission_parameters)
            .propose_and_print(
                ctx.ic_admin_executor().await?.execution(prop),
                ForumPostKind::AuthorizedSubnetsUpdate { body: summary },
            )
            .await
    }
}

fn construct_summary(
    subnets: &Arc<IndexMap<PrincipalId, Subnet>>,
    new_default_subnets: &BTreeSet<PrincipalId>,
    current_default_subnets: BTreeSet<PrincipalId>,
) -> anyhow::Result<String> {
    Ok(format!(
        "Updating the list of authorized subnets to:

| Subnet id | Subnet Type | Public |
| --------- | ----------- | ------ |
{}
",
        subnets
            .values()
            .map(|s| {
                let was_default = current_default_subnets.contains(&s.principal);
                let is_default = new_default_subnets.contains(&s.principal);
                format!(
                    "| {} | {} | {} |",
                    s.principal,
                    match &s.subnet_type {
                        SubnetType::Application => "Application",
                        SubnetType::System => "System",
                        SubnetType::VerifiedApplication => "Verified Application",
                    },
                    match (was_default, is_default) {
                        // The state doesn't change
                        (was_default, is_default) if was_default == is_default => was_default.to_string(),
                        // It changed from `was_default` to `is_excluded`
                        (was_default, is_default) => format!("~~{}~~ ⇒ {}", was_default, is_default),
                    },
                )
            })
            .join("\n")
    ))
}
