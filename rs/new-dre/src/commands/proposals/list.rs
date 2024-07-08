use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_governance::pb::v1::ListProposalInfo;
use itertools::Itertools;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

use super::Proposal;

#[derive(Args, Debug)]
pub struct List {
    /// Limit on the number of \[ProposalInfo\] to return. If no value is
    /// specified, or if a value greater than 100 is specified, 100
    /// will be used.
    #[clap(long, default_value = "100")]
    pub limit: u32,

    /// If specified, only return proposals that are strictly earlier than
    /// the specified proposal according to the proposal ID. If not
    /// specified, start with the most recent proposal.
    #[clap(long)]
    pub before_proposal: Option<u64>,

    /// Exclude proposals with a topic in this list. This is particularly
    /// useful to exclude proposals on the topics TOPIC_EXCHANGE_RATE and
    /// TOPIC_KYC which most users are not likely to be interested in
    /// seeing.
    #[clap(long)]
    pub exclude_topic: Vec<i32>,

    /// Include proposals that have a reward status in this list (see
    /// \[ProposalRewardStatus\] for more information). If this list is
    /// empty, no restriction is applied. For example, many users listing
    /// proposals will only be interested in proposals for which they can
    /// receive voting rewards, i.e., with reward status
    /// PROPOSAL_REWARD_STATUS_ACCEPT_VOTES.
    #[clap(long)]
    pub include_reward_status: Vec<i32>,

    /// Include proposals that have a status in this list (see
    /// \[ProposalStatus\] for more information). If this list is empty, no
    /// restriction is applied.
    #[clap(long)]
    pub include_status: Vec<i32>,

    /// Include all ManageNeuron proposals regardless of the visibility of the
    /// proposal to the caller principal. Note that exclude_topic is still
    /// respected even when this option is set to true.
    #[clap(long)]
    pub include_all_manage_neuron_proposals: Option<bool>,

    /// Omits "large fields" from the response. Currently only omits the
    /// `logo` and `token_logo` field of CreateServiceNervousSystem proposals. This
    /// is useful to improve download times and to ensure that the response to the
    /// request doesn't exceed the message size limit.
    #[clap(long)]
    pub omit_large_fields: Option<bool>,
}

impl ExecutableCommand for List {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let proposals = client
            .list_proposals(ListProposalInfo {
                limit: self.limit,
                before_proposal: self.before_proposal.as_ref().map(|p| ProposalId { id: *p }),
                exclude_topic: self.exclude_topic.clone(),
                include_reward_status: self.include_reward_status.clone(),
                include_status: self.include_status.clone(),
                include_all_manage_neuron_proposals: self.include_all_manage_neuron_proposals.clone(),
                omit_large_fields: self.omit_large_fields,
            })
            .await?
            .into_iter()
            .map(|p| {
                Proposal::try_from(p.clone())
                    .map(|r| serde_json::to_value(r).expect("cannot serialize to json"))
                    .unwrap_or_else(|_| serde_json::to_value(p).expect("cannot serialize to json"))
            })
            .collect_vec();

        let proposals = serde_json::to_string_pretty(&proposals).map_err(|e| anyhow::anyhow!("Couldn't serialize to string: {:?}", e))?;
        println!("{}", proposals);
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
