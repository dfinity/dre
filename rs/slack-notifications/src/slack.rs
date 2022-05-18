use candid::Deserialize;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_common::pb::v1::ProposalId;
use ic_nns_governance::pb::v1::ProposalInfo;
use itertools::Itertools;
use log::info;
use regex::Regex;
use reqwest::IntoUrl;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::convert::TryFrom;

const TRUSTED_NEURONS_TAG: &str = "<!subteam^S0200F4EYLF>";
const MAX_SUMMARY_LENGTH: usize = 2048;

#[derive(Debug, Serialize, Deserialize)]
struct NeuronSlackMapping {
    pub id: String,
    pub neuron: u64,
}

pub struct SlackHook<T: IntoUrl> {
    client: reqwest::Client,
    url: T,
    channel: Option<String>,
}

impl<T: IntoUrl + Clone> SlackHook<T> {
    pub fn new(url: T) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            url,
            channel: None,
        }
    }

    pub fn channel(mut self, channel: String) -> Self {
        self.channel = Some(channel);
        self
    }

    pub async fn send(&self, payload: &SlackPayload) -> Result<reqwest::Response, reqwest::Error> {
        let mut payload = payload.clone();
        if let Some(channel) = &self.channel {
            payload.payload["channel"] = channel.clone().into();
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

#[derive(Clone)]
pub struct SlackPayload {
    pub payload: Value,
}

impl Serialize for SlackPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.payload.serialize(serializer)
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

    let result = RE
        .captures(summary)
        .and_then(|c| c.name(&MOTIVATION_GROUP_NAME).map(|m| m.as_str()))
        .unwrap_or(summary)
        .to_string();

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

fn proposer_tag(proposer: NeuronId) -> Option<String> {
    let neurons = serde_yaml::from_str::<Vec<NeuronSlackMapping>>(include_str!("../conf/neurons-slack-mapping.yaml"))
        .expect("failed parsing neurons config");
    let neurons_blacklist = serde_yaml::from_str::<HashSet<u64>>(include_str!("../conf/proposer-blacklist.yaml"))
        .expect("failed parsing neurons config");

    if neurons_blacklist.contains(&proposer.id) {
        None
    } else {
        match neurons.iter().find(|nm| nm.neuron == proposer.id) {
            Some(nm) => Some(format!("<@{}>", nm.id)),
            None => Some(format!("Neuron {}", proposer.id)),
        }
    }
}

impl TryFrom<Vec<ProposalInfo>> for SlackPayload {
    type Error = anyhow::Error;
    // https://app.slack.com/block-kit-builder/T43F9UHS5#%7B%22blocks%22:%5B%5D%7D
    fn try_from(proposals: Vec<ProposalInfo>) -> Result<Self, anyhow::Error> {
        let message_groups = proposals.into_iter().group_by(|p| {
            (
                proposer_tag(p.proposer.clone().expect("proposer not set")),
                proposal_motivation(p),
            )
        });
        let message_groups = message_groups
            .into_iter()
            .filter_map(|((proposer_tag, motivation), group)| proposer_tag.map(|pt| ((pt, motivation), group)))
            .collect::<Vec<_>>();

        if message_groups.is_empty() {
            return Err(anyhow::anyhow!("no sendable proposals"));
        }

        let message = format!("{} please review the following proposal(s)", TRUSTED_NEURONS_TAG);
        Ok(Self {
            payload: json!({
                "text": message,
                "blocks": message_groups
                    .into_iter()
                    .fold(vec![json!({
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": message,
                        }
                    })], |acc, ((proposer_tag, motivation), group)| [
                        acc.as_slice(),
                        vec![
                            json!({
                                "type": "divider"
                            }),
                            json!({
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": format!(">{}", motivation.replace('\n', "\n>")),
                                },
                                "fields": group.map(|p|
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
                                        "text": format!("Proposed by {}", proposer_tag),
                                    }
                                ]
                            }),
                        ].as_slice()
                    ].concat().to_vec())
            }),
        })
    }
}
