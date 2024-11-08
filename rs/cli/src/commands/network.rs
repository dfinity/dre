use clap::Args;

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
#[clap(alias = "heal")]
pub struct Network {
    /// Heal the unhealthy subnets by replacing unhealthy nodes in them.
    #[clap(long)]
    pub heal: bool,

    /// Ensure that at least one node of each node operator is
    /// assigned to some (any) subnet. Node will only be assigned to a subnet if
    /// this does not worsen the decentralization of the target subnet.
    #[clap(long)]
    pub ensure_operator_nodes_assigned: bool,

    /// Skip provided subnets.
    #[clap(long, num_args(1..))]
    pub skip_subnets: Vec<String>,
}

impl ExecutableCommand for Network {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        let ic_admin = ctx.ic_admin().await?;
        let mut errors = vec![];
        let network_heal = self.heal || std::env::args().any(|arg| arg == "heal");
        if network_heal {
            let proposals = runner.network_heal(ctx.forum_post_link(), &self.skip_subnets).await?;
            for proposal in proposals {
                if let Err(e) = ic_admin.propose_run(proposal.cmd, proposal.opts).await {
                    errors.push(e);
                }
            }
        } else {
            log::info!("No network healing requested: ");
        }
        if self.ensure_operator_nodes_assigned {
            let proposals = runner
                .network_ensure_operator_nodes_assigned(ctx.forum_post_link(), &self.skip_subnets)
                .await?;
            for proposal in proposals {
                if let Err(e) = ic_admin.propose_run(proposal.cmd, proposal.opts).await {
                    errors.push(e);
                }
            }
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(anyhow::anyhow!("All errors received:\n{:?}", errors)),
        }
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        // At least one of the two options must be provided
        let network_heal = self.heal || std::env::args().any(|arg| arg == "heal");
        if !network_heal && !self.ensure_operator_nodes_assigned {
            cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "At least one of '--heal' or '--ensure-operator-nodes-assigned' must be specified.",
            )
            .exit()
        }
    }
}
