use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    io::Write,
};

use clap::Args;
use ic_protobuf::registry::firewall::v1::FirewallRule;
use ic_registry_keys::FirewallRulesScope;
use itertools::Itertools;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::{
    auth::AuthRequirement,
    ctx::DreContext,
    exe::{args::GlobalArgs, ExecutableCommand},
    forum::ForumPostKind,
    ic_admin::{IcAdminProposal, IcAdminProposalCommand, IcAdminProposalOptions},
    proposal_executors::{ProducesProposalResult, ProposalResponseWithId, RunnableViaIcAdmin},
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Args, Debug)]
pub struct Firewall {
    #[clap(long, default_value = Some("Proposal to modify firewall rules"))]
    pub title: Option<String>,

    #[clap(long, default_value = None, required = true)]
    pub summary: Option<String>,

    /// Ruleset scope: "global", "replica_nodes", "api_boundary_nodes", "subnet(SUBNET_ID)", "node(NODE_ID)"
    #[clap(long, default_value = None, required = true)]
    pub rules_scope: FirewallRulesScope,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for Firewall {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        let registry = ctx.registry().await;
        let firewall_ruleset = registry.firewall_rule_set(self.rules_scope.clone()).await?;

        let rules: BTreeMap<usize, &FirewallRule> = firewall_ruleset.entries.iter().enumerate().sorted_by(|a, b| a.0.cmp(&b.0)).collect();

        let mut builder = edit::Builder::new();
        let with_suffix = builder.suffix(".json");
        let pretty = serde_json::to_string_pretty(&rules).map_err(|e| anyhow::anyhow!("Error serializing ruleset to string: {:?}", e))?;
        let edited: BTreeMap<usize, FirewallRule>;
        loop {
            info!("Spawning edit window...");
            let edited_string = edit::edit_with_builder(pretty.clone(), with_suffix)?;
            match serde_json::from_str(&edited_string) {
                Ok(ruleset) => {
                    edited = ruleset;
                    break;
                }
                Err(e) => {
                    warn!("Couldn't parse the input you provided, please retry. Error: {:?}", e);
                }
            }
        }

        let mut added_entries: BTreeMap<usize, &FirewallRule> = BTreeMap::new();
        let mut updated_entries: BTreeMap<usize, &FirewallRule> = BTreeMap::new();
        for (key, rule) in edited.iter() {
            if let Some(old_rule) = rules.get(key) {
                if rule != *old_rule {
                    // Same key but different value meaning it was just updated
                    updated_entries.insert(*key, rule);
                }
                continue;
            }
            // Doesn't exist in old ones meaning it was just added
            added_entries.insert(*key, rule);
        }

        // Collect removed entries (keys from old set not present in new set)
        let removed_entries: BTreeMap<usize, &FirewallRule> = rules.into_iter().filter(|(key, _)| !edited.contains_key(key)).collect();

        let mut mods = FirewallRuleModifications::new();
        for (pos, rule) in added_entries.into_iter() {
            mods.addition(pos, rule.clone());
        }
        for (pos, rule) in updated_entries.into_iter() {
            mods.update(pos, rule.clone());
        }
        for (pos, rule) in removed_entries.into_iter() {
            mods.removal(pos, rule.clone());
        }

        let reverse_sorted = mods.reverse_sorted_and_batched();
        if reverse_sorted.is_empty() {
            info!("No modifications should be made");
            return Ok(());
        }
        let diff = serde_json::to_string_pretty(&reverse_sorted).unwrap();
        info!("Pretty printing diff:\n{}", diff);
        //TODO: adapt to use set-firewall config so we can modify more than 1 rule at a time

        match reverse_sorted.into_iter().last() {
            Some((_, mods)) => {
                Self::create_proposal(
                    &ctx,
                    mods,
                    self.title.clone(),
                    self.summary.clone(),
                    &self.submission_parameters,
                    &self.rules_scope,
                )
                .await
            }
            None => Err(anyhow::anyhow!("Expected to have one item for firewall rule modification")),
        }
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}

#[derive(Deserialize)]
struct FirewallTestResult {
    hash: String,
}

impl TryFrom<String> for FirewallTestResult {
    type Error = serde_json::Error;
    fn try_from(a: String) -> Result<Self, Self::Error> {
        serde_json::from_str(a.as_str())
    }
}

struct FirewallTestCommand {
    args: Vec<String>,
    #[allow(dead_code)]
    // Holds a reference to the temporary file so it's not deleted
    // until after proposal execution is done and the ref is dropped.
    tempfile: NamedTempFile,
}

impl FirewallTestCommand {
    fn new(
        modifications: Vec<FirewallRuleModification>,
        change_type: &FirewallRuleModificationType,
        rules_scope: &FirewallRulesScope,
        positions: &String,
    ) -> anyhow::Result<Self> {
        let mut file = NamedTempFile::new().map_err(|e| anyhow::anyhow!("Couldn't create temp file: {:?}", e))?;
        let args = [
            vec![format!("{}-firewall-rules", change_type), "--test".to_string(), rules_scope.to_string()],
            match change_type {
                FirewallRuleModificationType::Removal => {
                    vec![positions.to_string(), "none".to_string()]
                }
                _ => {
                    let rules = modifications.iter().map(|modif| modif.clone().rule_being_modified).collect::<Vec<_>>();
                    let serialized = serde_json::to_string(&rules).unwrap();
                    file.write_all(serialized.as_bytes())
                        .map_err(|e| anyhow::anyhow!("Couldn't write to tempfile: {:?}", e))?;
                    vec![file.path().to_str().unwrap().to_string(), positions.to_string(), "none".to_string()]
                }
            },
        ]
        .concat();
        Ok(FirewallTestCommand { args, tempfile: file })
    }
}

impl RunnableViaIcAdmin for FirewallTestCommand {
    type Output = FirewallTestResult;
    fn to_ic_admin_arguments(&self) -> anyhow::Result<Vec<String>> {
        IcAdminProposal::new(IcAdminProposalCommand::Raw(self.args.clone()), Default::default()).to_ic_admin_arguments()
    }
}

struct FirewallModifyCommand {
    test_command: FirewallTestCommand,
    summary: Option<String>,
    title: Option<String>,
    hash: String,
}

impl RunnableViaIcAdmin for FirewallModifyCommand {
    type Output = ProposalResponseWithId;
    fn to_ic_admin_arguments(&self) -> anyhow::Result<Vec<String>> {
        // Uses nearly the same arguments as the test argv.
        let mut final_args = self.test_command.args.clone();
        // Remove --test from head of args.
        let _ = final_args.remove(1);
        // Add the real hash to args.
        let last = final_args.last_mut().unwrap();
        *last = self.hash.to_string();
        IcAdminProposal::new(
            IcAdminProposalCommand::Raw(final_args),
            IcAdminProposalOptions {
                title: self.title.clone(),
                summary: self.summary.clone(),
                motivation: None,
            },
        )
        .to_ic_admin_arguments()
    }
}

impl ProducesProposalResult for FirewallModifyCommand {
    type ProposalResult = ProposalResponseWithId;
}

impl Firewall {
    async fn create_proposal(
        ctx: &DreContext,
        modifications: Vec<FirewallRuleModification>,
        title: Option<String>,
        summary: Option<String>,
        submission_parameters: &SubmissionParameters,
        firewall_rules_scope: &FirewallRulesScope,
    ) -> anyhow::Result<()> {
        let positions = modifications.iter().map(|modif| modif.position).join(",");
        let change_type = modifications[0].clone().change_type;

        let test_command = FirewallTestCommand::new(modifications, &change_type, firewall_rules_scope, &positions)?;

        let parsed = ctx
            .ic_admin_executor()
            .await?
            .run(&test_command, None)
            .await
            .map_err(|e| anyhow::anyhow!("Test for {}-firewall-rules failed: {:?}", change_type, e))?;

        let hash = parsed.hash;
        info!("Computed hash for firewall rule at position '{}': {}", positions, hash);

        // We should probably not be calling the print variant here, since
        // this is called in a loop by the caller of create_proposal.  Perhaps
        // we should summarize the proposals that were submitted, or the errors
        // that were returned.
        Submitter::from(submission_parameters)
            .propose_and_print(
                ctx.ic_admin_executor().await?.execution(FirewallModifyCommand {
                    test_command,
                    hash,
                    summary,
                    title,
                }),
                ForumPostKind::Generic,
            )
            .await
    }
}

#[derive(Clone, Serialize, PartialEq)]
pub enum FirewallRuleModificationType {
    Addition,
    Update,
    Removal,
}

impl Display for FirewallRuleModificationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Addition => write!(f, "add"),
            Self::Update => write!(f, "update"),
            Self::Removal => write!(f, "remove"),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct FirewallRuleModification {
    change_type: FirewallRuleModificationType,
    rule_being_modified: FirewallRule,
    position: usize,
}

impl FirewallRuleModification {
    pub fn addition(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Addition,
            rule_being_modified: rule,
            position,
        }
    }
    pub fn update(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Update,
            rule_being_modified: rule,
            position,
        }
    }
    pub fn removal(position: usize, rule: FirewallRule) -> Self {
        Self {
            change_type: FirewallRuleModificationType::Removal,
            rule_being_modified: rule,
            position,
        }
    }
}

pub struct FirewallRuleModifications {
    raw: Vec<FirewallRuleModification>,
}

impl FirewallRuleModifications {
    pub fn new() -> Self {
        FirewallRuleModifications { raw: vec![] }
    }

    pub fn addition(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::addition(position, rule))
    }

    pub fn update(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::update(position, rule))
    }

    pub fn removal(&mut self, position: usize, rule: FirewallRule) {
        self.raw.push(FirewallRuleModification::removal(position, rule))
    }

    pub fn reverse_sorted(&self) -> Vec<FirewallRuleModification> {
        let mut sorted = self.raw.to_vec();
        sorted.sort_by(|first, second| first.position.partial_cmp(&second.position).unwrap());
        sorted.reverse();
        sorted
    }

    pub fn reverse_sorted_and_batched(&self) -> Vec<(FirewallRuleModificationType, Vec<FirewallRuleModification>)> {
        let mut batches: Vec<(FirewallRuleModificationType, Vec<FirewallRuleModification>)> = vec![];
        let mut current_batch: Vec<FirewallRuleModification> = vec![];
        let mut modtype: Option<FirewallRuleModificationType> = None;
        for modif in self.reverse_sorted().iter() {
            if modtype.is_none() {
                modtype = Some(modif.clone().change_type);
            }
            if modtype.clone().unwrap() == modif.change_type {
                current_batch.push(modif.clone())
            } else {
                batches.push((current_batch[0].clone().change_type, current_batch));
                current_batch = vec![];
                modtype = Some(modif.clone().change_type);
            }
        }
        if !current_batch.is_empty() {
            batches.push((current_batch[0].clone().change_type, current_batch))
        }
        batches
    }
}
