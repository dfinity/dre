# linear-jira

linear-jira enables state syncing between Linear and Jira to enable teams to use Linear while still maintaing the state in Jira for any other stakeholders. Primary direction of syncing is Linear -> Jira, but some Jira -> Linear syncing is available.

## Development

Configure the environment variables by copying `.env.template` and modifying it to your needs.

**IMPORTANT: Please use `RELTEST` Jira project, and `RELT` Linear team for any local development.**

```shell
cp .env.template .env
```

Run a sync.

```shell
yarn dev
```

## Syncing overview

The syncing has the following mapping.

<!-- Generated with: https://www.tablesgenerator.com/markdown_tables -->

| Linear    | Jira              | Linear -> Jira     | Jira -> Linear     |
| --------- | ----------------- | ------------------ | ------------------ |
| Team      | Project           | :white_check_mark: | TBD                |
| Project   | Epic              | :white_check_mark: | TBD                |
| Issue     | Task              | :white_check_mark: | :white_check_mark: |
| Relations | Relations         | :white_check_mark: | :x:                |
| Sub-issue | Split-to relation | :white_check_mark: | :x:                |
| Labels    | Labels            | TBD                | TBD                |

## Features

### Linear to Jira issues syncing

All the Linear issues and projects are synced to Jira issues. Linear issues and sub-issues are created as Jira tasks, while projects are created as epics.

The field of the issues is synced continously on any updates to the Linear issue.

* :white_check_mark: Title
* :white_check_mark: Description
* :white_check_mark: State
* :white_check_mark: Assignee
* :white_check_mark: Reporter
* :white_check_mark: Creator
* :white_check_mark:/:heavy_minus_sign: Relations (blocks / duplicates / relates) (removals are not implemented)
* :white_check_mark:/:heavy_minus_sign: Children (removals are not implemented)
* :white_check_mark:/:heavy_minus_sign: Links (only Jira <-> Linear link is updated)

### Jira to Linear syncing

Issues (not Epics) created in Jira are created in Linear and are automatically managed by Linear in the future. If for some reason, you need to managed the ticket in Jira (e.g. assigning the ticket to a person outside of the team who's not using Linear), you can remove the `managed-by-linear` label from the Jira ticket. At that point, Linear will have `Managed by Jira` label. To reverse the action, simply remove the label from Linear.

## Missing features

* Cross-project / cross-team relations - primarily to avoid polluting any other projects by accident
* Relation removals - would require state persistence or comparing the entire state of both Linear and Jira
* Label syncing
