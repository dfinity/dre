# slack-notifications

Announces NNS proposals: it polls the governance canister and posts new
proposals, and proposals that failed to execute, to a chat channel.

Proposals are split into two streams. Motion proposals (topic 4) are the
**external** stream; everything else is the **internal** one. Each stream has
its own destination and moves between Slack and Open Messaging on its own.

## Where a stream is posted

For each stream, the first of these that is set wins:

1. its Open Messaging webhook, `OPEN_MESSAGING_WEBHOOK_URL_PROPOSALS_*`
2. its Slack channel, `SLACK_CHANNEL_PROPOSALS_*`, posted through `SLACK_URL`

A stream with neither is skipped, and its proposals are logged and dropped.
Unsetting a webhook moves that stream back to Slack, so the move can be rolled
back without a new image.

Open Messaging takes Slack's incoming-webhook format, but renders only an
attachment's title, title link, colour bar and mrkdwn text. Block Kit blocks,
which the Slack announcement is built from, and Slack's group and user
mentions do not survive the trip, so each group of proposals is posted there
as a single attachment: the `@trusted-neurons` and `@devrel` mentions are left
out, since there is no group to ping, and the proposer is named by neuron id
rather than by a `<@U…>` handle that would show up raw.

## Environment

### SLACK_URL

- *Description:* Address of a Slack incoming webhook. It is a secret.
- *Usage:* Posts every stream that is still on Slack. Not needed once both
  streams have an Open Messaging webhook.

### SLACK_CHANNEL_PROPOSALS_INTERNAL, SLACK_CHANNEL_PROPOSALS_EXTERNAL

- *Description:* Slack channel for the stream, e.g. `#nns-proposals`.
- *Usage:* Overrides the channel of the `SLACK_URL` webhook per stream.

### OPEN_MESSAGING_WEBHOOK_URL_PROPOSALS_INTERNAL, OPEN_MESSAGING_WEBHOOK_URL_PROPOSALS_EXTERNAL

- *Description:* Address of an Open Messaging incoming webhook,
  `https://<gateway>/api/hooks/<id>/<secret>`. It is a secret.
- *Usage:* When set, the stream is posted there instead of to Slack. The
  webhook names the channel, so no channel variable is needed. Create it in
  the channel's settings, under Integrations. Both variables may hold the same
  webhook, which puts both streams in one channel.

### NETWORK, NNS_URLS

- *Description:* Target network, and its NNS URLs. `NNS_URLS` is required for
  testnets and optional for mainnet and staging.
