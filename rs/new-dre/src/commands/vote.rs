use std::{collections::HashSet, io::Write, time::Duration};

use chrono::Local;
use clap::Args;
use humantime::{format_duration, parse_duration};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_governance::pb::v1::ProposalInfo;
use log::info;
use spinners::{Spinner, Spinners};

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Vote {
    /// Override default accepted proposers
    /// These are the proposers which proposals will
    /// be automatically voted on
    ///
    /// By default: DRE + automation neuron 80
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "PROPOSER_ID",
        default_value = "80,39,40,46,58,61,77"
    )]
    pub accepted_neurons: Vec<u64>,

    /// Override default topics to vote on
    /// Use with caution! This is subcommand is intended to be used
    /// only by DRE in processes of rolling out new versions,
    /// everything else should be double checked manually
    ///
    /// By default: SubnetReplicaVersionManagement
    #[clap(long, use_value_delimiter = true, value_delimiter = ',', value_name = "PROPOSER_ID", default_value = "12")]
    pub accepted_topics: Vec<i32>,

    /// Override default sleep time
    #[clap(long, default_value = "60s", value_parser = parse_duration)]
    pub sleep_time: Duration,
}

impl ExecutableCommand for Vote {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client: GovernanceCanisterWrapper = ctx.create_canister_client()?.into();

        let mut voted_proposals = HashSet::new();
        info!("Starting the voting loop...");

        loop {
            let proposals = client.get_pending_proposals().await?;
            let proposals: Vec<&ProposalInfo> = proposals
                .iter()
                .filter(|p| {
                    self.accepted_topics.contains(&p.topic)
                        && self.accepted_neurons.contains(&p.proposer.unwrap().id)
                        && !voted_proposals.contains(&p.id.unwrap().id)
                })
                .collect();

            // Clear last line in terminal
            print!("\x1B[1A\x1B[K");
            std::io::stdout().flush().unwrap();

            for proposal in proposals {
                let datetime = Local::now();
                info!(
                    "{} Voting on proposal {} (topic {:?}, proposer {}) -> {}",
                    datetime,
                    proposal.id.unwrap().id,
                    proposal.topic(),
                    proposal.proposer.unwrap_or_default().id,
                    proposal.proposal.clone().unwrap().title.unwrap()
                );

                let response = client.register_vote(ctx.ic_admin().neuron.neuron_id, proposal.id.unwrap().id).await?;
                info!("{}", response);
                voted_proposals.insert(proposal.id.unwrap().id);
            }

            let mut sp = Spinner::with_timer(
                Spinners::Dots12,
                format!(
                    "Sleeping {} before another check for pending proposals...",
                    format_duration(self.sleep_time)
                ),
            );
            let sleep_func = tokio::time::sleep(self.sleep_time);
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C, exiting...");
                    sp.stop();
                    break;
                }
                _ = sleep_func => {
                    sp.stop_with_message("Done sleeping, checking for pending proposals...".into());
                    continue
                }
            }
        }

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
