# Who really is Dr. DRE (`@dr-dre`)?

To enhance our team's effectiveness, we have implemented a weekly rotation system. Each week, a designated team member takes responsibility for managing routine and unexpected operations. This document provides an overview of the on-call responsibilities and includes important links to resources that will support you in this role.

## Where to find the rotation

The rotation schedules can be found on our [Jira Team Operations page](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/on-call). There are two schedules, both following the same round-robin system:

1. **DRE Alerts**: Handles automatic paging related to our infrastructure.  
2. **DRE Ops Rotation**: Determines who will act as `@dr-dre` (our Slack handle).

??? question "Why are there two schedules?"
    The two-schedule system was designed to separate responsibilities and ensure balance.  

    - **DRE Alerts** focuses on managing infrastructure alerts and operates only during working hours, as we don’t adhere to any strict SLA/SLO requirements.  
    - **DRE Ops Rotation** handles Slack pings and general team operations.  

??? question "I am not getting paged for alerts?"
    We use the [Jira cloud app](https://www.atlassian.com/software/jira/mobile-app) for on-call and rotations.

    To set it up follow the document on [Notion](https://www.notion.so/dfinityorg/Setting-up-Jira-App-for-oncall-rotation-17def9d9b80c80439418ec7e60a32a15)!

## Regular activities

As Dr. DRE, your role for the week involves taking on several responsibilities. These include, but are not limited to:

### 1. **Follow through the release process**

The release process [is documented here](https://www.notion.so/dfinityorg/IC-OS-release-technical-aspects-1e3c3274ba4d406ebe222aa6eb569e3a).  In short:

* Follow the schedule presented on the [rollout dashboard](https://rollout-dashboard.ch1-rel1.dfinity.network/).
* Follow the statuses visible in [airflow](https://airflow.ch1-rel1.dfinity.network/dags/rollout_ic_os_to_mainnet_subnets/grid).
* Vote on the proposals being submitted by the automation.
* Cut a new release on Thrusday and create any additional feature builds.
* If needed, create ordinary hotfixes or [security hotfixes](https://docs.google.com/document/d/19iYuAxwvWFbxfM3AdhydA5GzfaCITNhueSDhkxKevYQ/edit?tab=t.0#heading=h.i2ciz6mp3ue0) for that week.
* In-depth explaination of the release process can be found on [Notion](https://www.notion.so/dfinityorg/IC-OS-release-technical-aspects-1e3c3274ba4d406ebe222aa6eb569e3a#9621e1dc378c4b3ba28c9d2d1ac5b3a7).

??? tip "Regular week"
    Usually most of this work boils down to running
    ```bash
    dre vote
    ```

### 2. **Review alerts for our clusters**

* All alerts that our clusters send are aggregated in our [Jira ops board](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/alerts?view=list&query=responders%3A+og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53)
* Heartbeats are present [here](https://dfinity.atlassian.net/jira/ops/teams/og-a6d6c0d5-2641-4c54-8a2c-5860ef5e8f53/heartbeats)

??? tip "What should I do if there are alerts?"
    
    - It's not expected that every alert can be resolved immediately or by a single team member.
    - The key objective is to maintain the stability of our clusters.
    - Evaluate the alert based on its severity and the affected cluster to determine if further action is required.
    - Escalate or address issues as needed to ensure operations continue smoothly.

### 3. **Answer in our team's slack channel**
    
* [`#eng-dre`](https://dfinity.enterprise.slack.com/archives/C05LD0CEAHY): General channel for activities
* [`#eng-release`](https://dfinity.enterprise.slack.com/archives/C01DB8MQ5M1): Questions related to release process
* [`#eng-observability`](https://dfinity.enterprise.slack.com/archives/CGZ4YGN4S): Questions related to our observability

??? question "But I don't know the answers to all questions"

    - It’s perfectly fine not to have all the answers.
    - Take the initiative to investigate the issue and see how you can assist.
    - If you’re unable to resolve the question, redirect it to the appropriate team member.
    - The primary goal is to support the organization and relieve pressure on the rest of the team during your on-call week.

### 4. **Submit requested proposals**

* Replace dead nodes
* Help in on-boarding or off-boarding of datacenters and node providers
* Firewall rule modifications
* Any other requested proposals

??? tip "Tooling"
    For all regular ops we have sufficient tooling implemented in our `dre` tool. For all new proposals and specific scenarios it is your responsibility to add them to the tooling as the new use cases come.

### 5. **Monitor status and health of CI**  

- **Weekly dependency upgrade job**:  

   - A [GitHub Action](https://github.com/dfinity/dre/actions/workflows/update-dependencies.yaml) runs weekly to automatically upgrade dependencies.  
   - While some weeks result in straightforward updates, others may require manual intervention due to API changes or other breaking updates.  

- **Your responsibility**:  

   - Review and address any issues with the generated pull request.  
   - Ensure the fixes are implemented and attempt to merge the PR into the repository.  
   - Maintaining compatibility between the IC repo and our repo reduces friction and ensures our tooling operates smoothly.  

### 6. **Handover operations**  

- If there are any pending tasks or unresolved operations, it is your responsibility to inform the next on-call team member.  
- Provide clear details on what needs to be addressed and any context that might help them pick up where you left off.
