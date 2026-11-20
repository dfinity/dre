use candid::Deserialize;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_nns_governance::pb::v1::Topic;
use ic_nns_governance_api::ProposalInfo;
use itertools::Itertools;
use log::warn;
use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::convert::TryFrom;

use crate::notify::Destination;

const TRUSTED_NEURONS_TAG: &str = "<!subteam^S0200F4EYLF>";
const DEVREL_TAG: &str = "<!subteam^S04AHQT37RQ>";
const RELEASE_TEAM_TAG: &str = ""; // Can be changed to the following to mention @release-engs on each proposal:
// "<!subteam^S02CF4KKZ7U>";
const DRDRE_TAG: &str = "<!subteam^S05GPUNS7EX>";
const RELEASE_AUTOMATION_NEURON_ID: u64 = 80;
const MAX_SUMMARY_LENGTH: usize = 2048;
const SLACK_CHANNEL_ENV_INTERNAL: &str = "SLACK_CHANNEL_PROPOSALS_INTERNAL";
const SLACK_CHANNEL_ENV_EXTERNAL: &str = "SLACK_CHANNEL_PROPOSALS_EXTERNAL";
const OPEN_MESSAGING_WEBHOOK_ENV_INTERNAL: &str = "OPEN_MESSAGING_WEBHOOK_URL_PROPOSALS_INTERNAL";
const OPEN_MESSAGING_WEBHOOK_ENV_EXTERNAL: &str = "OPEN_MESSAGING_WEBHOOK_URL_PROPOSALS_EXTERNAL";

#[derive(Debug, Serialize, Deserialize)]
struct NeuronSlackMapping {
    pub neuron_id: u64,
    pub slack_id: String,
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

    let result = RE
        .captures(summary)
        .and_then(|c| c.name(&MOTIVATION_GROUP_NAME).map(|m| m.as_str()))
        .unwrap_or(summary)
        .to_string();

    let result_len = result.chars().count();
    if result_len > MAX_SUMMARY_LENGTH {
        let end = result.chars().map(|c| c.len_utf8()).take(MAX_SUMMARY_LENGTH).sum();
        let result = &result[0..end];
        format!("{} <{} more characters truncated>", result, result_len - MAX_SUMMARY_LENGTH)
    } else {
        result
    }
}

pub fn proposal_url(id: ProposalId) -> String {
    format!("https://dashboard.internetcomputer.org/proposal/{}", id.id)
}

pub fn proposal_link_markdown(id: ProposalId) -> String {
    format!("<{}|*{}*>", proposal_url(id), id.id)
}

fn proposer_mention(proposer: NeuronId) -> Option<String> {
    let neurons =
        serde_yaml::from_str::<Vec<NeuronSlackMapping>>(include_str!("../conf/neurons-slack-mapping.yaml")).expect("failed parsing neurons config");
    let neurons_blacklist =
        serde_yaml::from_str::<HashSet<u64>>(include_str!("../conf/proposer-blacklist.yaml")).expect("failed parsing neurons config");

    if neurons_blacklist.contains(&proposer.id) {
        None
    } else {
        match neurons.iter().find(|nm| nm.neuron_id == proposer.id) {
            Some(nm) => Some(format!("<@{}>", nm.slack_id)),
            None => Some(format!("Neuron {}", proposer.id)),
        }
    }
}

fn alert_mention(proposer: &NeuronId, topic: Topic) -> String {
    if topic == Topic::IcOsVersionDeployment {
        DRDRE_TAG.to_string()
    } else if proposer.id == RELEASE_AUTOMATION_NEURON_ID {
        RELEASE_TEAM_TAG.to_string()
    } else {
        TRUSTED_NEURONS_TAG.to_string()
    }
}

#[derive(Clone)]
pub struct SlackMessage {
    pub destination: Destination,
    pub alert_mention: String,
    pub proposer_mention: String,
    /// The proposer, for destinations where a Slack handle does not render.
    pub proposer_neuron_id: u64,
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
    /// Whether every proposal of the group failed, which decides the wording
    /// and, on Open Messaging, the colour of the bar.
    pub fn all_failed(&self) -> bool {
        self.proposals
            .iter()
            .all(|p| ProposalStatus::try_from(p.status).expect("failed to parse proposal status") == ProposalStatus::Failed)
    }

    /// The proposer, named without Slack markup.
    ///
    /// The neuron mapping holds Slack handles, not names, and a `<@U…>` handle
    /// renders raw outside Slack, so the neuron id is used instead.
    pub fn proposer_name(&self) -> String {
        format!("Neuron {}", self.proposer_neuron_id)
    }

    pub fn render_payload(&self) -> Value {
        let message = self.alert_mention.clone()
            + if self.all_failed() {
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

/// Where a proposal's announcement goes.
///
/// Topic 4 is for Motion proposals, which are classified as external;
/// everything else is internal. Each of the two streams moves to Open
/// Messaging on its own, as soon as a webhook is set for it, and falls back to
/// its Slack channel otherwise, so unsetting the webhook moves it back.
fn destination_for_proposal(proposal: &ProposalInfo) -> Option<Destination> {
    let (open_messaging_env, slack_channel_env) = if proposal.topic == 4 {
        (OPEN_MESSAGING_WEBHOOK_ENV_EXTERNAL, SLACK_CHANNEL_ENV_EXTERNAL)
    } else {
        (OPEN_MESSAGING_WEBHOOK_ENV_INTERNAL, SLACK_CHANNEL_ENV_INTERNAL)
    };

    if let Some(webhook_url) = std::env::var(open_messaging_env).ok().filter(|url| !url.is_empty()) {
        return Some(Destination::OpenMessaging { webhook_url });
    }
    std::env::var(slack_channel_env)
        .ok()
        .filter(|channel| !channel.is_empty())
        .map(|channel| Destination::Slack { channel })
}

impl TryFrom<Vec<ProposalInfo>> for MessageGroups {
    type Error = anyhow::Error;
    // https://app.slack.com/block-kit-builder/T43F9UHS5#%7B%22blocks%22:%5B%5D%7D
    fn try_from(proposals: Vec<ProposalInfo>) -> Result<Self, anyhow::Error> {
        let message_groups = proposals
            .into_iter()
            .chunk_by(|p| {
                let topic = Topic::try_from(p.topic).expect("failed to parse topic");
                let proposer = p.proposer.expect("proposer not set");
                (
                    destination_for_proposal(p),
                    alert_mention(&proposer, topic),
                    proposer_mention(proposer),
                    proposer.id,
                    proposal_motivation(p),
                    topic,
                )
            })
            .into_iter()
            .filter_map(
                |((destination, alert_mention, proposer_mention, proposer_neuron_id, motivation, topic), group)| {
                    let proposals = group.collect::<Vec<_>>();
                    // A stream with neither a webhook nor a Slack channel has
                    // nowhere to go: drop it rather than fail the whole batch.
                    let Some(destination) = destination else {
                        warn!(
                            "no destination configured for {} proposal(s) on {:?}; skipping them",
                            proposals.len(),
                            topic
                        );
                        return None;
                    };
                    proposer_mention.map(|proposer_mention| SlackMessage {
                        destination,
                        alert_mention: alert_mention + if topic == Topic::SnsAndCommunityFund { DEVREL_TAG } else { "" },
                        proposer_mention,
                        proposer_neuron_id,
                        motivation,
                        proposals,
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
    use ic_nns_governance_api::proposal::Action;
    use ic_nns_governance_api::{Motion, Proposal};
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard};
    use std::time::SystemTime;

    const INTERNAL_CHANNEL: &str = "#nns-proposals-test-internal";
    const EXTERNAL_CHANNEL: &str = "#nns-proposals-test-external";
    const INTERNAL_WEBHOOK: &str = "https://gateway.example/api/hooks/abc/s3cr3t-internal";
    const EXTERNAL_WEBHOOK: &str = "https://gateway.example/api/hooks/def/s3cr3t-external";

    /// Routing reads the environment, which is shared by the whole test
    /// process, so the tests that depend on it take this lock for their
    /// duration and set every variable they rely on, including the ones they
    /// need unset.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct RoutingEnv {
        internal_channel: Option<&'static str>,
        external_channel: Option<&'static str>,
        internal_webhook: Option<&'static str>,
        external_webhook: Option<&'static str>,
    }

    impl RoutingEnv {
        /// Both streams on Slack, which is how the app ran before Open
        /// Messaging.
        fn slack_only() -> Self {
            Self {
                internal_channel: Some(INTERNAL_CHANNEL),
                external_channel: Some(EXTERNAL_CHANNEL),
                internal_webhook: None,
                external_webhook: None,
            }
        }

        fn apply(self) -> MutexGuard<'static, ()> {
            // A test that panicked while holding the lock has poisoned it;
            // the variables are set again below, so the state it left behind
            // does not matter.
            let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            unsafe {
                std::env::set_var("SLACK_URL", "http://localhost");
                for (name, value) in [
                    (SLACK_CHANNEL_ENV_INTERNAL, self.internal_channel),
                    (SLACK_CHANNEL_ENV_EXTERNAL, self.external_channel),
                    (OPEN_MESSAGING_WEBHOOK_ENV_INTERNAL, self.internal_webhook),
                    (OPEN_MESSAGING_WEBHOOK_ENV_EXTERNAL, self.external_webhook),
                ] {
                    match value {
                        Some(value) => std::env::set_var(name, value),
                        None => std::env::remove_var(name),
                    }
                }
            }
            guard
        }
    }

    /// The Slack channel a message goes to, failing the test if it is bound
    /// for Open Messaging instead.
    fn slack_channel(message: &SlackMessage) -> &str {
        match &message.destination {
            Destination::Slack { channel } => channel,
            Destination::OpenMessaging { .. } => panic!("expected a Slack destination, got Open Messaging"),
        }
    }

    /// The webhook a message goes to, failing the test if it is bound for
    /// Slack instead.
    fn open_messaging_webhook(message: &SlackMessage) -> &str {
        match &message.destination {
            Destination::OpenMessaging { webhook_url } => webhook_url,
            Destination::Slack { channel } => panic!("expected an Open Messaging destination, got the Slack channel {}", channel),
        }
    }

    fn gen_test_proposal(proposal_id: u64, proposer: u64, summary: &str, topic: i32) -> ProposalInfo {
        ProposalInfo {
            id: Some(ProposalId { id: proposal_id }),
            proposer: Some(NeuronId { id: proposer }),
            reject_cost_e8s: 1000000000,
            proposal: Some(Proposal {
                title: Some("A Reasonable Title".to_string()),
                summary: String::from(summary),
                action: Some(Action::Motion(Motion {
                    motion_text: "me like proposals".to_string(),
                })),
                ..Default::default()
            }),
            proposal_timestamp_seconds: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            topic,
            status: 1, // AcceptVotes
            ballots: HashMap::new(),
            latest_tally: None,
            decided_timestamp_seconds: 0,
            executed_timestamp_seconds: 0,
            reward_event_round: 0,
            reward_status: 0,
            failed_timestamp_seconds: 0,
            failure_reason: None,
            deadline_timestamp_seconds: None,
            derived_proposal_information: None,
            total_potential_voting_power: None,
            success_value: None,
        }
    }

    #[test]
    fn grouping_into_1_message() {
        let _env = RoutingEnv::slack_only().apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary", 5), gen_test_proposal(1001, 77, "summary", 5)];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 1);
        let msg1 = &message_groups[0];
        assert_eq!(slack_channel(msg1), INTERNAL_CHANNEL);
        assert_eq!(msg1.proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(msg1.motivation, "summary".to_string());
    }

    #[test]
    fn grouping_into_2_message() {
        let _env = RoutingEnv::slack_only().apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary 1", 5), gen_test_proposal(1001, 77, "summary 2", 5)];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(slack_channel(&message_groups[0]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[0].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(slack_channel(&message_groups[1]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[1].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[1].motivation, "summary 2".to_string());
    }

    #[test]
    fn grouping_into_2_message_2_slack_channels() {
        let _env = RoutingEnv::slack_only().apply();
        let proposals = vec![
            gen_test_proposal(1000, 77, "summary 1", 5),
            gen_test_proposal(1001, 77, "summary 1", 4), // Motion proposal --> external channel
        ];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(slack_channel(&message_groups[0]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[0].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(slack_channel(&message_groups[1]), EXTERNAL_CHANNEL);
        assert_eq!(message_groups[1].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[1].motivation, "summary 1".to_string());
    }

    #[test]
    fn mention_trusted_neurons() {
        let _env = RoutingEnv::slack_only().apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary 1", 5), gen_test_proposal(1001, 80, "summary 1", 7)];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(slack_channel(&message_groups[0]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[0].alert_mention, TRUSTED_NEURONS_TAG);
        assert_eq!(message_groups[0].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
        assert_eq!(slack_channel(&message_groups[1]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[1].alert_mention, RELEASE_TEAM_TAG);
        assert_eq!(message_groups[1].proposer_mention, "Neuron 80");
        assert_eq!(message_groups[1].motivation, "summary 1".to_string());
    }

    #[test]
    fn mention_devrel() {
        let _env = RoutingEnv::slack_only().apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary 1", Topic::SnsAndCommunityFund.into())];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 1);
        assert_eq!(slack_channel(&message_groups[0]), INTERNAL_CHANNEL);
        assert_eq!(message_groups[0].alert_mention, format!("{}{}", TRUSTED_NEURONS_TAG, DEVREL_TAG));
        assert_eq!(message_groups[0].proposer_mention, "<@U04CMGD6XDH>");
        assert_eq!(message_groups[0].motivation, "summary 1".to_string());
    }

    #[test]
    fn a_webhook_moves_only_its_own_stream() {
        let _env = RoutingEnv {
            internal_webhook: Some(INTERNAL_WEBHOOK),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let proposals = vec![
            gen_test_proposal(1000, 77, "summary 1", 5), // internal --> Open Messaging
            gen_test_proposal(1001, 77, "summary 1", 4), // Motion, external --> still Slack
        ];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(open_messaging_webhook(&message_groups[0]), INTERNAL_WEBHOOK);
        assert_eq!(slack_channel(&message_groups[1]), EXTERNAL_CHANNEL);
    }

    #[test]
    fn both_streams_can_share_one_open_messaging_channel() {
        let _env = RoutingEnv {
            internal_webhook: Some(INTERNAL_WEBHOOK),
            external_webhook: Some(INTERNAL_WEBHOOK),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary 1", 5), gen_test_proposal(1001, 77, "summary 1", 4)];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(open_messaging_webhook(&message_groups[0]), INTERNAL_WEBHOOK);
        assert_eq!(open_messaging_webhook(&message_groups[1]), INTERNAL_WEBHOOK);
    }

    #[test]
    fn each_stream_can_have_its_own_open_messaging_channel() {
        let _env = RoutingEnv {
            internal_webhook: Some(INTERNAL_WEBHOOK),
            external_webhook: Some(EXTERNAL_WEBHOOK),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let proposals = vec![gen_test_proposal(1000, 77, "summary 1", 5), gen_test_proposal(1001, 77, "summary 1", 4)];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        assert_eq!(message_groups.len(), 2);
        assert_eq!(open_messaging_webhook(&message_groups[0]), INTERNAL_WEBHOOK);
        assert_eq!(open_messaging_webhook(&message_groups[1]), EXTERNAL_WEBHOOK);
    }

    #[test]
    fn an_empty_webhook_leaves_the_stream_on_slack() {
        let _env = RoutingEnv {
            internal_webhook: Some(""),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let message_groups = MessageGroups::try_from(vec![gen_test_proposal(1000, 77, "summary 1", 5)])
            .unwrap()
            .message_groups;
        assert_eq!(slack_channel(&message_groups[0]), INTERNAL_CHANNEL);
    }

    #[test]
    fn proposals_with_nowhere_to_go_are_skipped() {
        let _env = RoutingEnv {
            internal_channel: None,
            external_channel: None,
            internal_webhook: None,
            external_webhook: None,
        }
        .apply();
        // Nothing is configured, so the batch yields no messages rather than
        // a message with no destination.
        assert!(MessageGroups::try_from(vec![gen_test_proposal(1000, 77, "summary 1", 5)]).is_err());
    }

    #[test]
    fn open_messaging_payload_is_one_attachment() {
        let _env = RoutingEnv {
            internal_webhook: Some(INTERNAL_WEBHOOK),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let proposals = vec![
            gen_test_proposal(1000, 80, "Motivation: a good reason", 5),
            gen_test_proposal(1001, 80, "Motivation: a good reason", 5),
        ];
        let message_groups = MessageGroups::try_from(proposals).unwrap().message_groups;
        let payload = crate::open_messaging::render_payload(&message_groups[0]);

        let attachments = payload["attachments"].as_array().expect("no attachments");
        assert_eq!(attachments.len(), 1);
        let attachment = &attachments[0];
        assert_eq!(attachment["title"], "Please review 2 proposal(s)");
        assert_eq!(attachment["title_link"], "https://dashboard.internetcomputer.org/proposal/1000");
        assert_eq!(attachment["color"], "warning");

        let text = attachment["text"].as_str().expect("no text");
        assert!(text.contains("a good reason"), "the motivation is missing from {}", text);
        assert!(text.contains("<https://dashboard.internetcomputer.org/proposal/1000|*1000*>"));
        assert!(text.contains("<https://dashboard.internetcomputer.org/proposal/1001|*1001*>"));
        // The proposer is named by neuron, since a Slack handle renders raw.
        assert!(text.contains("Neuron 80"), "the proposer is missing from {}", text);
        // Block Kit is not rendered by the gateway, so it must not be sent.
        assert!(payload["blocks"].is_null());
    }

    #[test]
    fn failed_proposals_get_a_red_bar() {
        let _env = RoutingEnv {
            internal_webhook: Some(INTERNAL_WEBHOOK),
            ..RoutingEnv::slack_only()
        }
        .apply();
        let mut proposal = gen_test_proposal(1000, 80, "summary", 5);
        proposal.status = ProposalStatus::Failed as i32;
        let message_groups = MessageGroups::try_from(vec![proposal]).unwrap().message_groups;
        let payload = crate::open_messaging::render_payload(&message_groups[0]);

        assert_eq!(payload["attachments"][0]["title"], "1 proposal(s) failed");
        assert_eq!(payload["attachments"][0]["color"], "danger");
    }
}
