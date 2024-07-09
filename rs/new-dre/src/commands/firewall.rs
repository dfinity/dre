use clap::Args;
use ic_registry_keys::FirewallRulesScope;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Firewall {
    #[clap(long, default_value = Some("Proposal to modify firewall rules"))]
    pub title: Option<String>,

    #[clap(long, default_value = None, required = true)]
    pub summary: Option<String>,

    /// Ruleset scope: "global", "replica_nodes", "api_boundary_nodes", "subnet(SUBNET_ID)", "node(NODE_ID)"
    #[clap(long, default_value = None, required = true)]
    pub rules_scope: FirewallRulesScope,
}

impl ExecutableCommand for Firewall {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
