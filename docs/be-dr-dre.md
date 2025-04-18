# Who really is Dr. DRE (`@dr-dre`)?

To enhance our team's effectiveness, we have implemented a weekly rotation system. Each week, a designated team member takes responsibility for managing routine and unexpected operations. This document provides an overview of the on-call responsibilities and includes important links to resources that will support you in this role.

## Where to find the rotation

The rotation schedules can be found on our [Jira Team Operations page](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/on-call) (requires DFINITY Jira access which can be obtained through Okta). There are two schedules, both following the same round-robin system:

1. **DRE Alerts**: Handles automatic paging related to our infrastructure.
2. **DRE Ops Rotation**: Determines who will act as `@dr-dre` (our Slack handle).

??? question "Why are there two schedules?"
    The two-schedule system was designed to separate responsibilities and ensure balance.

    - **DRE Alerts** focuses on managing infrastructure alerts and operates only during working hours, as we don’t adhere to any strict SLA/SLO requirements.
    - **DRE Ops Rotation** handles Slack pings and general team operations.

??? question "I am not getting paged for alerts?"
    We use the [Jira cloud app](https://www.atlassian.com/software/jira/mobile-app) for on-call and rotations.

    To set it up follow the document on [Notion](https://www.notion.so/dfinityorg/Setting-up-Jira-App-for-oncall-rotation-17def9d9b80c80439418ec7e60a32a15).

## Regular activities

As Dr. DRE, your role for the week involves taking on several responsibilities. These include, but are not limited to:

### 1. **Follow through the IC OS release process**

The release process [is documented in detail here](https://www.notion.so/dfinityorg/IC-OS-release-technical-aspects-1e3c3274ba4d406ebe222aa6eb569e3a).  In short:

* Follow the schedule presented on the [rollout dashboard](https://rollout-dashboard.ch1-rel1.dfinity.network/).  If problems arise, diagnose using the low-level statuses from [Airflow](https://airflow.ch1-rel1.dfinity.network/dags/rollout_ic_os_to_mainnet_subnets/grid) (the dashboard also links directly to the problem task in Airflow).
* Cut a new GuestOS & HostOS release on Thursday, and create any additional feature builds [as per the spreadsheet](https://docs.google.com/spreadsheets/d/1ZcYB0gWjbgg7tFgy2Fhd3llzYlefJIb0Mik75UUrSXM/edit) as well as [security hotfixes](https://docs.google.com/document/d/19iYuAxwvWFbxfM3AdhydA5GzfaCITNhueSDhkxKevYQ/edit?tab=t.0#heading=h.i2ciz6mp3ue0).
* Ensure team engineers review the release notes through Friday.
* Ensure the release controller submits GuestOS & HostOS version elect proposals on Friday -- *not earlier*, to allow sufficient time for community and DFINITY voters to review and vote without rush.
* In-depth explanation of the release process can be found on [Notion](https://www.notion.so/dfinityorg/IC-OS-release-technical-aspects-1e3c3274ba4d406ebe222aa6eb569e3a#9621e1dc378c4b3ba28c9d2d1ac5b3a7).

??? tip "Regular week"
    Usually most of this work boils down to running
    ```bash
    dre vote
    ```

### 2. **Review alerts for our clusters**

* All alerts that our clusters send are aggregated in our [Jira ops board](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/alerts?view=list&query=responders%3A%20og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53%20AND%20%28status%3A%20%22snoozed%22%20OR%20status%3A%20%22acknowledged%22%20OR%20status%3A%20%22open%22%29).
* Heartbeats are present [here](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/heartbeats).

??? tip "What should I do if there are alerts?"

    - It's not expected that every alert can be resolved immediately or by a single team member.
    - The key objective is to maintain the stability of our clusters.
    - Evaluate the alert based on its severity and the affected cluster to determine if further action is required.
    - Escalate or address issues as needed to ensure operations continue smoothly.

### 3. **Handle all notifications and answer all questions asked in the team's slack channels**

* [`#eng-dre`](https://dfinity.enterprise.slack.com/archives/C05LD0CEAHY): General channel for activities
* [`#eng-release`](https://dfinity.enterprise.slack.com/archives/C01DB8MQ5M1): Questions related to release process
* [`#eng-release-bots`](https://dfinity.enterprise.slack.com/archives/C02NQC42C2F): Automations send important notifications to this channel, which you must handle
* [`#eng-observability`](https://dfinity.enterprise.slack.com/archives/CGZ4YGN4S): Questions related to our observability

??? question "But I don't know the answers to all questions"

    - It’s perfectly fine not to have all the answers.
    - Take the initiative to investigate the issue and see how you can assist.
    - If you’re unable to resolve the question, redirect it to the appropriate team member.
    - The primary goal is to support the organization and relieve pressure on the rest of the team during your on-call week.

### 4. **Submit requested proposals**

All requested proposals must:

1. Be registered as a ticket under the [DRE Ops Rotation queue](https://dfinity.atlassian.net/browse/DRE-350)
2. Include clear requirements and expected outcomes
3. Be followed through in a timely manner based on priority

Typical types of requested proposals are:

* Help in on-boarding or off-boarding of datacenters and node providers
* Firewall rule modifications
* Node rewards adjustment proposals (see _Handoff operations_ below)
* Any other requested proposals

??? tip "Tooling"
    For all regular ops we have sufficient tooling implemented in our `dre` tool. For all new proposals and specific scenarios it is your responsibility to add them to the tooling as the new use cases come.

### 5. **Submit proposals conventionally submitted once a week**

* Replace dead nodes
* Mainnet topology proposals, such as `dre network --heal --optimize --ensure-operator-nodes-unassigned --ensure-operator-nodes-assigned --remove-cordoned-nodes` or a subset of these operations. The operations are still not polished enough to be run automatically.
* Provider reward adjustment proposals, if any are needed that week. Please ask in `#eng-dre` if you don't know if any are needed.

Please [register proposals as tickets under the DRE Ops Rotation queue](https://dfinity.atlassian.net/browse/DRE-350), so adoption and progress can be tracked, and context can be observed by your teammates.

### 6. **Monitor status and health of CI**

- **Weekly dependency upgrade jobs**:

   - A [GitHub Action](https://github.com/dfinity/dre/actions/workflows/update-dependencies.yaml) runs weekly to automatically upgrade dependencies.
   - Dependabot also issues PRs regularly.
   - While some weeks result in straightforward updates, others may require manual intervention due to API changes or other breaking updates.
   - Review and address any issues with the generated pull request
     - Find the automation PRs [here](https://github.com/dfinity/dre/issues?q=is%3Apr+is%3Aopen+author%3Aapp%2Fpr-automation-bot-public).
     - Find the Dependabot PRs [here](https://github.com/dfinity/dre/issues?q=is%3Apr+is%3Aopen+author%3Aapp%2Fdependabot).
   - Ensure the fixes are implemented and attempt to merge the PR into the repository.
   - Maintaining compatibility between the IC repo and our repo reduces friction and ensures our tooling operates smoothly.

### 7. **Drive progress on the DRE Ops Rotation task queue**

Our [DRE Ops rotation dashboard](https://dfinity.atlassian.net/jira/dashboards/10331) lets you view the queue.  The queue exists to keep track of work falling under the Dr. DRE umbrella that may span multiple days or weeks.  It contains a list of child tickets that you need to work on.

Tend to the queue at least once a day.  Read and heed the guidelines in the umbrella epic.  Here is a brief summary (which is not a substitute for [reading the guidelines](https://dfinity.atlassian.net/browse/DRE-350)):

* Record (as tickets of type task) multi-day work under the umbrella of the DRE Ops Rotation, with the task queue ticket as the new ticket's epic.
* Drive progress on tasks that are not blocked.
* Mark blocked tasks as blocked.
* Record completion of tasks.
* Provide enough context there for your teammates to pick up ongoing work the week after.
* Move tickets that change scope *out* of the queue and *into* its own epic or project.

### 8. **Handoff operations**

- If there are any pending tasks or unresolved operations, it is your responsibility to inform the next on-call team member.
- Provide clear details on what needs to be addressed and any context that might help them pick up where you left off.
- Pass on information about node rewards adjustments requested to the next on-call team member.

The [DRE Ops Rotation dashboard](https://dfinity.atlassian.net/jira/dashboards/10331) is an invaluable aid in getting yourself in context as well as providing context to your teammates.
