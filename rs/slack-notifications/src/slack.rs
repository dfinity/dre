use candid::Deserialize;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_nns_governance::pb::v1::{proposal, ProposalInfo, Topic};
use itertools::Itertools;
use log::info;
use regex::Regex;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use reqwest::IntoUrl;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::convert::TryFrom;

const TRUSTED_NEURONS_TAG: &str = "<!subteam^S0200F4EYLF>";
const DEVREL_TAG: &str = "<!subteam^S04AHQT37RQ>";
const RELEASE_TEAM_TAG: &str = ""; // Can be changed to the following to mention @release-engs on each proposal:
                                   // "<!subteam^S02CF4KKZ7U>";
const RELEASE_AUTOMATION_NEURON_ID: u64 = 80;
const MAX_SUMMARY_LENGTH: usize = 2048;
const SLACK_CHANNEL_ENV_INTERNAL: &str = "SLACK_CHANNEL_PROPOSALS_INTERNAL";
const SLACK_CHANNEL_ENV_EXTERNAL: &str = "SLACK_CHANNEL_PROPOSALS_EXTERNAL";

#[derive(Debug, Serialize, Deserialize)]
struct NeuronSlackMapping {
    pub neuron_id: u64,
    pub slack_id: String,
}

pub struct SlackHook<T: IntoUrl> {
    client: reqwest::Client,
    url: T,
}

impl<T: IntoUrl + Clone> SlackHook<T> {
    pub fn new(url: T) -> Self {
        let client = reqwest::Client::new();
        Self { client, url }
    }

    pub async fn send(&self, slack_message: &SlackMessage) -> Result<reqwest::Response, reqwest::Error> {
        let mut payload = slack_message.render_payload();
        if let Some(channel) = &slack_message.slack_channel {
            payload["channel"] = json!(channel);
        } else {
            panic!("No slack channel provided");
        }
        info!(
            "Sending slack payload: {}",
            serde_json::to_string(&payload).unwrap_or_default()
        );

        self.client
            .post(self.url.clone())
            .json(&payload)
            .send()
            .await?
            .error_for_status()
    }
}

fn proposal_motivation(proposal_info: &ProposalInfo) -> String {
    lazy_static! {
        static ref MOTIVATION_GROUP_NAME: &'static str = "motivation";
        static ref RE: Regex = Regex::new(format!("Motivation: (?P<{}>.+)", *MOTIVATION_GROUP_NAME).as_str()).unwrap();
    }
    let summary = proposal_info
        .proposal
        .as_ref()
        .map(|p| p.summary.as_ref())
        .unwrap_or("no proposal summary");

    let mut result = RE
        .captures(summary)
        .and_then(|c| c.name(&MOTIVATION_GROUP_NAME).map(|m| m.as_str()))
        .unwrap_or(summary)
        .to_string();

    // For subnet membership changes (node replacements), add a link to the internal
    // dashboard where the decentralization data can be reviewed before voting.
    if proposal_info.topic == Topic::SubnetManagement as i32 {
        if let Some(proposal) = proposal_info.proposal.as_ref() {
            if let Some(proposal::Action::ExecuteNnsFunction(action)) = &proposal.action {
                if let Ok(change_membership_proposal) =
                    candid::decode_one::<ChangeSubnetMembershipPayload>(&action.payload)
                {
                    let subnet_id = change_membership_proposal.subnet_id;
                    result.push_str(&format!(
                            "\nDecentralization changes from this proposal can be reviewed on the <https://dashboard.internal.dfinity.network/network/mainnet/subnet/{}/change|internal release dashboard>",
                            subnet_id
                        ));
                }
            }
        }
    }

    let result_len = result.chars().count();
    if result_len > MAX_SUMMARY_LENGTH {
        let end = result.chars().map(|c| c.len_utf8()).take(MAX_SUMMARY_LENGTH).sum();
        let result = &result[0..end];
        format!(
            "{} <{} more characters truncated>",
            result,
            result_len - MAX_SUMMARY_LENGTH
        )
    } else {
        result
    }
}

fn proposal_link_markdown(id: ProposalId) -> String {
    format!(
        "<https://dashboard.internetcomputer.org/proposal/{}|*{}*>",
        id.id, id.id
    )
}

fn proposer_mention(proposer: NeuronId) -> Option<String> {
    let neurons = serde_yaml::from_str::<Vec<NeuronSlackMapping>>(include_str!("../conf/neurons-slack-mapping.yaml"))
        .expect("failed parsing neurons config");
    let neurons_blacklist = serde_yaml::from_str::<HashSet<u64>>(include_str!("../conf/proposer-blacklist.yaml"))
        .expect("failed parsing neurons config");

    if neurons_blacklist.contains(&proposer.id) {
        None
    } else {
        match neurons.iter().find(|nm| nm.neuron_id == proposer.id) {
            Some(nm) => Some(format!("<@{}>", nm.slack_id)),
            None => Some(format!("Neuron {}", proposer.id)),
        }
    }
}

fn alert_mention(proposer: &NeuronId) -> String {
    if proposer.id == RELEASE_AUTOMATION_NEURON_ID {
        RELEASE_TEAM_TAG.to_string()
    } else {
        TRUSTED_NEURONS_TAG.to_string()
    }
}

#[derive(Clone)]
pub struct SlackMessage {
    pub slack_channel: Option<String>,
    pub alert_mention: String,
    pub proposer_mention: String,
    pub motivation: String,
    pub proposals: Vec<ProposalInfo>,
}

impl Serialize for SlackMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.render_payload().serialize(serializer)
    }
}

impl SlackMessage {
    pub fn render_payload(&self) -> Value {
        let message = self.alert_mention.clone()
            + if self.proposals.iter().all(|p| {
                ProposalStatus::from_i32(p.status).expect("failed to parse proposal status") == ProposalStatus::Failed
            }) {
                "the following proposal(s) failed"
            } else {
                "please review the following proposal(s)"
            };

        // https://app.slack.com/block-kit-builder/T43F9UHS5#%7B%22blocks%22:%5B%5D%7D
        json!({
            "text": message,
            "blocks": vec![
                    json!({
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": message,
                        }
                    }),
                    json!({
                            "type": "divider"
                        }),
                        json!({
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!(">{}", self.motivation.replace('\n', "\n>")),
                            },
                            "fields": self.proposals.iter().map(|p|
                                json!({
                                    "type": "mrkdwn",
                                    "text": proposal_link_markdown(p.id.expect("no proposal id")),
                                })
                            ).collect::<Vec<_>>(),
                        }),
                        json!({
                            "type": "context",
                            "elements": [
                                {
                                    "type": "mrkdwn",
                                    "text": format!("Proposed by {}", self.proposer_mention),
                                }
                            ]
                        }),
                    ]
        })
    }
}

pub struct MessageGroups {
    pub message_groups: Vec<SlackMessage>,
}

fn slack_channel_for_proposal(proposal: &ProposalInfo) -> Option<String> {
    // Topic 4 is for Motion proposals
    // Motion proposals are classified as external, everything else is internal
    if proposal.topic == 4 {
        if let Ok(channel) = std::env::var(SLACK_CHANNEL_ENV_EXTERNAL) {
            return Some(channel);
        }
    } else if let Ok(channel) = std::env::var(SLACK_CHANNEL_ENV_INTERNAL) {
        return Some(channel);
    }
    None
}

impl TryFrom<Vec<ProposalInfo>> for MessageGroups {
    type Error = anyhow::Error;
    // https://app.slack.com/block-kit-builder/T43F9UHS5#%7B%22blocks%22:%5B%5D%7D
    fn try_from(proposals: Vec<ProposalInfo>) -> Result<Self, anyhow::Error> {
        let message_groups = proposals
            .into_iter()
            .group_by(|p| {
                (
                    slack_channel_for_proposal(p),
                    alert_mention(p.proposer.as_ref().expect("No NeuronId in the proposal")),
                    proposer_mention(p.proposer.expect("proposer not set")),
                    proposal_motivation(p),
                    p.topic(),
                )
            })
            .into_iter()
            .filter_map(
                |((slack_channel, alert_mention, proposer_mention, motivation, topic), group)| {
                    proposer_mention.map(|proposer_mention| {
                        let proposals = group.collect::<Vec<_>>();
                        SlackMessage {
                            slack_channel,
                            alert_mention: alert_mention
                                + if topic == Topic::SnsAndCommunityFund {
                                    DEVREL_TAG
                                } else {
                                    ""
                                },
                            proposer_mention,
                            motivation,
                            proposals,
                        }
                    })
                },
            )
            .collect::<Vec<_>>();

        if message_groups.is_empty() {
            return Err(anyhow::anyhow!("no sendable proposals"));
        }

        Ok(MessageGroups { message_groups })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    use ic_nns_governance::pb::v1::proposal;
    use ic_nns_governance::pb::v1::Motion;
    use ic_nns_governance::pb::v1::Proposal;

    fn gen_test_proposal(proposal_id: u64, proposer: u64, summary: &str, topic: i32) -> ProposalInfo {
        ProposalInfo {
            id: Some(ProposalId { id: proposal_id }),
            proposer: Some(NeuronId { id: proposer }),
            reject_cost_e8s: 1000000000,
            proposal: Some(Proposal {
                title: Some("A Reasonable Title".to_string()),
                summary: String::from(summary),
                action: Some(proposal::Action::Motion(Motion {
                    motion_text: "me like proposals".to_string(),
                })),
                ..Default::default()
            }),
            proposal_timestamp_seconds: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            topic,
            status: 1, // AcceptVotes
            ..Default::default()
        }
    }

    #[test]
    fn grouping_into_1_message() {
        let proposals = vec![
            gen_test_proposal(1000, 40, "summary", 5),
            gen_test_proposal(1001, 40, "summary", 5),
        ];
        std::env::set_var("SLACK_URL", "http://localhost");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_INTERNAL", "#nns-proposals-test-internal");
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 1);
        let msg1 = &message_groups[0];
        assert_eq!(msg1.slack_channel.as_ref().unwrap(), "#nns-proposals-test-internal");
        assert_eq!(msg1.proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(msg1.motivation, "summary".to_string());
    }

    #[test]
    fn grouping_into_2_message() {
        let proposals = vec![
            gen_test_proposal(1000, 40, "summary 1", 5),
            gen_test_proposal(1001, 40, "summary 2", 5),
        ];
        std::env::set_var("SLACK_URL", "http://localhost");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_INTERNAL", "#nns-proposals-test-internal");
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(
            message_groups[0].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(message_groups[0].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(
            message_groups[1].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(message_groups[1].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[1].motivation, "summary 2".to_string());
    }

    #[test]
    fn grouping_into_2_message_2_slack_channels() {
        let proposals = vec![
            gen_test_proposal(1000, 40, "summary 1", 5),
            gen_test_proposal(1001, 40, "summary 1", 4), // Motion proposal --> external channel
        ];
        std::env::set_var("SLACK_URL", "http://localhost");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_INTERNAL", "#nns-proposals-test-internal");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_EXTERNAL", "#nns-proposals-test-external");
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(
            message_groups[0].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(message_groups[0].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(
            message_groups[1].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-external"
        );
        assert_eq!(message_groups[1].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[1].motivation, "summary 1".to_string());
    }

    #[test]
    fn mention_trusted_neurons() {
        let proposals = vec![
            gen_test_proposal(1000, 40, "summary 1", 5),
            gen_test_proposal(1001, 80, "summary 1", 7),
        ];
        std::env::set_var("SLACK_URL", "http://localhost");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_INTERNAL", "#nns-proposals-test-internal");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_EXTERNAL", "#nns-proposals-test-external");
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(
            message_groups[0].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(message_groups[0].alert_mention, TRUSTED_NEURONS_TAG);
        assert_eq!(message_groups[0].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(
            message_groups[1].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(message_groups[1].alert_mention, RELEASE_TEAM_TAG);
        assert_eq!(message_groups[1].proposer_mention, "Neuron 80");
        assert_eq!(message_groups[1].motivation, "summary 1".to_string());
    }

    #[test]
    fn mention_devrel() {
        let proposals = vec![gen_test_proposal(
            1000,
            40,
            "summary 1",
            Topic::SnsAndCommunityFund.into(),
        )];
        std::env::set_var("SLACK_URL", "http://localhost");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_INTERNAL", "#nns-proposals-test-internal");
        std::env::set_var("SLACK_CHANNEL_PROPOSALS_EXTERNAL", "#nns-proposals-test-external");
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 1);
        assert_eq!(
            message_groups[0].slack_channel.as_ref().unwrap(),
            "#nns-proposals-test-internal"
        );
        assert_eq!(
            message_groups[0].alert_mention,
            format!("{}{}", TRUSTED_NEURONS_TAG, DEVREL_TAG)
        );
        assert_eq!(message_groups[0].proposer_mention, "<@URT5Z7VDZ>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
    }
}
