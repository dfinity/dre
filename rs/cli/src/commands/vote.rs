use std::{collections::HashSet, io::Write, time::Duration};

use clap::Args;
use humantime::{format_duration, parse_duration};
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_governance::pb::v1::ProposalInfo;
use log::info;
use spinners::{Spinner, Spinners};

use super::{AuthRequirement, ExecutableCommand};
use crate::desktop_notify::DesktopNotifier;

#[derive(Args, Debug)]
pub struct Vote {
    /// Override default accepted proposers
    /// These are the proposers which proposals will
    /// be automatically voted on
    ///
    /// By default: DRE + automation neuron 80 + Rüdiger Birkner
    #[clap(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        value_name = "PROPOSER_ID",
        default_value = "80,39,40,46,58,61,77,17511507705568200227"
    )]
    pub accepted_neurons: Vec<u64>,

    /// Override default topics to vote on
    /// Use with caution! This is subcommand is intended to be used
    /// only by DRE in processes of rolling out new versions,
    /// everything else should be double checked manually
    ///
    /// By default: IcOsVersionDeployment
    #[clap(long, use_value_delimiter = true, value_delimiter = ',', default_value = "12")]
    pub accepted_topics: Vec<i32>,

    /// Sleep time between voting cycles.  If set to 0s,
    /// only one voting cycle will take place.
    #[clap(long, default_value = "60s", value_parser = parse_duration)]
    pub sleep_time: Duration,
}

impl ExecutableCommand for Vote {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client: GovernanceCanisterWrapper = ctx.create_ic_agent_canister_client(None)?.into();

        let mut voted_proposals = HashSet::new();

        if self.sleep_time != Duration::from_secs(0) {
            DesktopNotifier::send_info("DRE vote: starting", "Starting the voting loop...");
        }

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
            // No need to panic if standard out doesn't flush (e.g. /dev/null).
            let _ = std::io::stdout().flush();

            for proposal in proposals {
                DesktopNotifier::send_info(
                    "DRE vote: voting",
                    &format!(
                        "Voting on proposal {} (topic {:?}, proposer {}) -> {}",
                        proposal.id.unwrap().id,
                        proposal.topic(),
                        proposal.proposer.unwrap_or_default().id,
                        proposal.proposal.clone().unwrap().title.unwrap()
                    ),
                );

                let prop_id = proposal.id.unwrap().id;
                if !ctx.is_dry_run() {
                    let response = match client.register_vote(ctx.ic_admin().neuron().neuron_id, proposal.id.unwrap().id).await {
                        Ok(response) => format!("Voted successfully: {}", response),
                        Err(e) => {
                            DesktopNotifier::send_critical(
                                "DRE vote: error",
                                &format!(
                                    "Error voting on proposal {} (topic {:?}, proposer {}) -> {}",
                                    prop_id,
                                    proposal.topic(),
                                    proposal.proposer.unwrap_or_default().id,
                                    e
                                ),
                            );
                            format!("Error voting: {}", e)
                        }
                    };
                    info!("{}", response);
                } else {
                    info!("Would have voted for proposal {}", prop_id)
                }
                voted_proposals.insert(prop_id);
            }

            if self.sleep_time == Duration::from_secs(0) {
                break;
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

    fn validate(&self, _cmd: &mut clap::Command) {}
}
