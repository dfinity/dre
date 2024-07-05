use clap::Args;
use ic_registry_keys::FirewallRulesScope;

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
