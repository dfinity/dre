import { LinearClient, Issue, Project, LinearDocument, Connection, IssueRelation, User, LinearFetch } from '@linear/sdk'
import { env } from 'process';
import { readFileSync, writeFileSync, existsSync, unlinkSync } from 'fs';
import { inspect } from 'util';
import { Version3Client, Version3Models } from 'jira.js';
import _, { orderBy } from 'lodash';

import adf2md from 'adf-to-md';

import { JSONTransformer } from '@atlaskit/editor-json-transformer';
import { MarkdownTransformer } from '@atlaskit/editor-markdown-transformer';
import { DateComparator, Maybe, PaginationOrderBy } from '@linear/sdk/dist/_generated_documents';

const jsonTransformer = new JSONTransformer();
const markdownTransformer = new MarkdownTransformer();

const jiraProject = env.JIRA_PROJECT!;
const linearTeam = env.LINEAR_TEAM!;
const checkpointFile = env.CHECKPOINT_FILE ?? "checkpoint.txt";
const lastAttemptFile = env.LAST_ATTEMPT_FILE ?? "last-attempt.txt";

const LINEAR_LINK_TITLE = 'Linear';

const linearClient = new LinearClient({
    apiKey: env.LINEAR_API_KEY,
})

const jiraClient = new Version3Client({
    host: 'https://dfinity.atlassian.net',
    authentication: {
        basic: {
            email: env.JIRA_USER_EMAIL!,
            apiToken: env.JIRA_USER_API_KEY!,
        },
    },
});

const JIRA_ISSUE_LINK_IMPORTED_TITLE = 'Original issue in Jira';
const JIRA_ISSUE_LINK_TITLE = 'Jira';


enum JiraIssueType {
    Epic = "Epic",
    Task = "Task",
}

interface JiraSyncableLinearEntity {
    id: string;
    description?: string;
    url: string;
    summary: () => string;
    jiraIssueType: () => JiraIssueType;
    stateName: () => Promise<string | undefined>;
    jiraKey: () => Promise<string | undefined>;
    createLinearLink: (jiraIssueKey: string) => Promise<void>;
    jiraEpic: () => Promise<string | undefined>;
    assigneeEmail: () => Promise<string | undefined>;
    creatorEmail: () => Promise<string | undefined>;
    parentJiraKey: () => Promise<string | undefined>;
    userCommentsUntil: (until: Date) => Promise<({ id: string, body: string, authorDisplayName: string })[]>;
}

declare module '@linear/sdk' {
    interface Issue extends JiraSyncableLinearEntity { }
    interface Project extends JiraSyncableLinearEntity { }
}

function jiraIssueUrl(jiraIssueKey: string) {
    return `https://dfinity.atlassian.net/browse/${jiraIssueKey}`
}

Issue.prototype.summary = function (this: Issue): string {
    return this.title
}

Issue.prototype.jiraIssueType = function (this: Issue): JiraIssueType {
    return JiraIssueType.Task
}

Issue.prototype.stateName = async function (this: Issue): Promise<string | undefined> {
    return (this.state)?.then(s => {
        switch (s.name) {
            case "Todo":
                return "To Do"
            default:
                return s.name
        }
    })
}

Issue.prototype.jiraKey = async function (this: Issue): Promise<string | undefined> {
    return this.attachments().then(a => a.nodes.find(a => a.title === JIRA_ISSUE_LINK_TITLE || a.title === JIRA_ISSUE_LINK_IMPORTED_TITLE)?.url?.split('/').pop())
}

Issue.prototype.createLinearLink = async function (this: Issue, jiraIssueKey: string): Promise<void> {
    await linearClient.createAttachment({
        groupBySource: false,
        iconUrl: "https://dfinity.atlassian.net/favicon-family.ico",
        title: JIRA_ISSUE_LINK_TITLE,
        url: jiraIssueUrl(jiraIssueKey),
        issueId: this.id,
    })
}

Issue.prototype.jiraEpic = async function (this: Issue): Promise<string | undefined> {
    return (await (await this.project)?.links())?.nodes.find(a => a.label === JIRA_ISSUE_LINK_TITLE || a.label === JIRA_ISSUE_LINK_IMPORTED_TITLE)?.url?.split('/').pop()
}

Issue.prototype.assigneeEmail = async function (this: Issue): Promise<string | undefined> {
    return (await this.assignee)?.email
}

Issue.prototype.creatorEmail = async function (this: Issue): Promise<string | undefined> {
    return (await this.creator)?.email
}

Issue.prototype.parentJiraKey = async function (this: Issue): Promise<string | undefined> {
    return await (await this.parent)?.jiraKey()
}

Issue.prototype.userCommentsUntil = async function (this: Issue, until: Date): Promise<({ id: string, body: string, authorDisplayName: string })[]> {
    return await Promise.all(await this.comments({
        filter: {
            user: {
                isMe: {
                    eq: false
                }
            },
            updatedAt: {
                gt: until,
            }
        }
    }).then(c => loadAllPagedNodes(c)).then(c => c.map(c => c.user!.then(u => ({ ...c, authorDisplayName: u.name })))))
        .then(c => c.sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime()))
}


Project.prototype.summary = function (this: Project): string {
    return this.name
}

Project.prototype.jiraIssueType = function (this: Project): JiraIssueType {
    return JiraIssueType.Epic
}

Project.prototype.stateName = async function (this: Project): Promise<string | undefined> {
    let state = this.state.charAt(0).toUpperCase() + this.state.slice(1);
    switch (state) {
        case "Planned":
            return "To Do"
        case "Completed":
            return "Done"
        case "Paused":
            return "Blocked"
        case "Started":
            return "In Progress"
        default:
            return state
    }
}

Project.prototype.jiraKey = async function (this: Project): Promise<string | undefined> {
    return this.links().then(a => a.nodes.find(a => a.label === JIRA_ISSUE_LINK_TITLE || a.label === JIRA_ISSUE_LINK_IMPORTED_TITLE)?.url?.split('/').pop())
}

Project.prototype.createLinearLink = async function (this: Project, jiraIssueKey: string): Promise<void> {
    await linearClient.createProjectLink({
        label: JIRA_ISSUE_LINK_TITLE,
        url: jiraIssueUrl(jiraIssueKey),
        projectId: this.id,
    })
}

Project.prototype.jiraEpic = async function (this: Project): Promise<string | undefined> {
    return undefined
}

Project.prototype.assigneeEmail = async function (this: Project): Promise<string | undefined> {
    return (await this.lead)?.email
}

Project.prototype.creatorEmail = async function (this: Project): Promise<string | undefined> {
    return (await this.creator)?.email
}

Project.prototype.parentJiraKey = async function (this: Project): Promise<string | undefined> {
    return undefined
}

Project.prototype.userCommentsUntil = async function (this: Project, _: Date): Promise<({ id: string, body: string, authorDisplayName: string })[]> {
    return []
}

function jiraToLinearStateName(state: string, resolution?: string): string {
    if (resolution == "Done") {
        return "Done"
    } else if (resolution) {
        return "Canceled"
    } else if (state == "To Do") {
        return "Todo"
    } else {
        return state
    }
}

function jiraResolutionForState(state: string): string | undefined {
    switch (state) {
        case "Canceled":
            return "Won't Do";
        case "Done":
            return "Done";
        default:
            return undefined
    }
}

function descriptionLinearIdTag(linearId: string): string {
    return `Linear ID: ${linearId}`
}

async function loadAllPagedNodes<T>(pagedResults: Connection<T>): Promise<T[]> {
    return pagedResults.nodes.concat(
        pagedResults.pageInfo.hasNextPage
            ? await loadAllPagedNodes(await pagedResults.fetchNext())
            : []
    );
}

async function syncLinearEntities<T extends JiraSyncableLinearEntity>(
    jiraUsers: Version3Models.User[],
    linearUsers: User[],
    getEntities: (variables?: LinearDocument.Exact<{
        orderBy?: LinearDocument.Maybe<PaginationOrderBy>;
        filter: {
            updatedAt?: Maybe<DateComparator>;
        }
    }>) => LinearFetch<Connection<T>>,
    checkpoint: Date,
) {
    let paged = (await getEntities({
        orderBy: LinearDocument.PaginationOrderBy.UpdatedAt,
        filter: {
            updatedAt: {
                gt: checkpoint
            }
        }
    }));
    let entities = await loadAllPagedNodes(paged);
    let results = await Promise.all(entities.map(async (entity) => {
        console.log(`Syncing ${entity.url}`);
        try {
            let [
                linkedJiraKey,
                entityEpic,
                jiraAssignee,
            ] = await Promise.all([
                entity.jiraKey(),
                entity.jiraEpic(),
                entity.assigneeEmail().then(email => jiraUsers.find(u => u.emailAddress === email)?.accountId),
            ]);

            let jiraFields: any = {
                summary: entity.summary(),
                project: {
                    key: jiraProject,
                },
                assignee: {
                    id: jiraAssignee,
                },
                description: jsonTransformer.encode(markdownTransformer.parse(`${entity.description || ""}\n\n${descriptionLinearIdTag(entity.id)}`)),
                ...(entity.jiraIssueType() === JiraIssueType.Epic ? { customfield_10010: entity.summary() } : {}),
                ...(entity.jiraIssueType() === JiraIssueType.Task && entityEpic ? { customfield_10013: entityEpic } : {})
            };
            let jiraIssueKey = (
                linkedJiraKey ||
                (
                    (await jiraClient.issueSearch.searchForIssuesUsingJql({
                        jql: `summary ~ ${JSON.stringify(entity.summary())} AND description ~ "${descriptionLinearIdTag(entity.id)}" AND type = "${entity.jiraIssueType()}" AND project = "${jiraProject}" ORDER BY created DESC`,
                    })).issues?.find(_ => true)?.key
                ) || (
                    await jiraClient.issues.createIssue({
                        fields: {
                            reporter: {
                                id: await entity.creatorEmail().then(email => jiraUsers.find(u => u.emailAddress === email)?.accountId),
                            },
                            issuetype: {
                                name: entity.jiraIssueType(),
                            },
                            ...jiraFields,
                        }
                    }).then(i => i.key)
                )
            )!;

            if (!jiraAssignee) {
                const jiraIssue = await jiraClient.issues.getIssue({
                    issueIdOrKey: jiraIssueKey,
                });
                if (!linearUsers.some(u => u.email === jiraIssue.fields.assignee?.emailAddress)) {
                    delete jiraFields['assignee'];
                }
            }

            // Link Linear issue back to Jira
            if (!linkedJiraKey) {
                await entity.createLinearLink(jiraIssueKey);
            }

            // Link the Jira issue back to Linear if not set already
            let remoteLink = (await jiraClient.issueRemoteLinks.getRemoteIssueLinks({
                issueIdOrKey: jiraIssueKey,
            }) as Version3Models.RemoteIssueLink[]).find(l => l.object?.title === LINEAR_LINK_TITLE);
            const linearEntityUrlNoDescription = encodeURI(entity.url);
            if (!remoteLink) {
                await jiraClient.issueRemoteLinks.createOrUpdateRemoteIssueLink({
                    issueIdOrKey: jiraIssueKey,
                    object: {
                        url: linearEntityUrlNoDescription,
                        title: LINEAR_LINK_TITLE,
                    }
                });
            }

            // Update comments
            let linearComments = await entity?.userCommentsUntil(checkpoint);
            if (linearComments) {
                let jiraComments = await jiraClient.issueComments.getComments({ issueIdOrKey: jiraIssueKey, expand: "renderedBody" }).then(r => r.comments?.filter(c => c.author?.emailAddress === env.JIRA_USER_EMAIL));
                for (let linearComment of linearComments) {
                    let fields: any = {
                        body: jsonTransformer.encode(markdownTransformer.parse(`${linearComment.authorDisplayName} commented on Linear:\n\n${linearComment.body || ""}\n\n${descriptionLinearIdTag(linearComment.id)}`)),
                        issueIdOrKey: jiraIssueKey,
                    }
                    let existingJiraComment = jiraComments?.find(c => c.renderedBody?.includes(descriptionLinearIdTag(linearComment.id)));
                    if (existingJiraComment) {
                        await jiraClient.issueComments.updateComment({
                            ...fields,
                            id: existingJiraComment.id!,
                        })
                    } else {
                        await jiraClient.issueComments.addComment({
                            ...fields,
                        });
                    }
                }
            }

            // Update summary and description
            await jiraClient.issues.editIssue({
                issueIdOrKey: jiraIssueKey,
                fields: {
                    ...jiraFields,
                    description: jsonTransformer.encode(markdownTransformer.parse(entity.description || " ")),
                }
            })

            // Update the state of the ticket
            let stateName = await entity.stateName();
            if (stateName) {
                let jiraIssue = await jiraClient.issues.getIssue({
                    issueIdOrKey: jiraIssueKey,
                })
                if (jiraIssue.fields.status.name !== stateName) {
                    let transitions = await jiraClient.issues.getTransitions({
                        issueIdOrKey: jiraIssueKey,
                    });
                    let transition = transitions.transitions?.find(t => t.name == (stateName == "Canceled" ? "Done" : stateName));
                    if (transition) {
                        let resolution = jiraResolutionForState(stateName);
                        await jiraClient.issues.doTransition({
                            issueIdOrKey: jiraIssueKey,
                            transition: {
                                id: transition.id,
                            },
                            ...(resolution ? {
                                fields: {
                                    resolution: {
                                        name: resolution
                                    },
                                }
                            } : {})
                        })
                    } else {
                        console.log(`Unable to update Jira ticket "${jiraIssueKey}" to state "${stateName}": transition not found (available: ${transitions.transitions?.map(t => t.name)})`)
                    }
                }
            }

            let parentJiraKey = await entity.parentJiraKey();
            if (parentJiraKey) {
                await jiraClient.issueLinks.linkIssues({
                    inwardIssue: {
                        key: parentJiraKey,
                    },
                    outwardIssue: {
                        key: jiraIssueKey,
                    },
                    type: {
                        name: "Issue split",
                    },
                })
            }
        } catch (e) {
            console.log(`Failed to update entity ${entity.url}`, inspect(e, { showHidden: false, depth: 2, colors: false }));
            return e
        }
    }));

    if (results.find(r => r != undefined)) {
        throw "Some entities failed to sync"
    }
}

function capitalizeFirstLetter(string: string) {
    return string.charAt(0).toUpperCase() + string.slice(1);
}

function linearJiraRelationTypeMapping(type: string) {
    switch (type) {
        case "related":
            return "Relates"
    }

    return capitalizeFirstLetter(type)
}

async function syncLinearRelations(relations: IssueRelation[]) {
    let results = await Promise.all(relations.map(async (r) => {
        try {
            let [issue, relatedIssue] = await Promise.all([
                r.issue?.then(async (i) => ({ ...i, team: await i.team, jiraKey: await i.jiraKey() })),
                r.relatedIssue?.then(async (i) => ({ ...i, team: await i.team, jiraKey: await i.jiraKey() })),
            ]);

            // Only sync relations completely within the syncing team
            if (issue?.team?.key !== linearTeam || relatedIssue?.team?.key !== linearTeam) {
                return
            }

            await jiraClient.issueLinks.linkIssues({
                inwardIssue: {
                    key: issue.jiraKey,
                },
                outwardIssue: {
                    key: relatedIssue.jiraKey,
                },
                type: {
                    name: linearJiraRelationTypeMapping(r.type),
                },
            })
        } catch (e) {
            return e
        }
    }));
    if (results.find(r => r !== undefined)) {
        throw "Some relations failed to sync";
    }
}

function descriptionJiraIdTag(jiraId: string): string {
    return `Jira ID: ${jiraId}`
}

async function syncJiraIssuesToLinear(checkpoint: Date, until: Date, linearUsers: User[]) {
    const jiraSearchMinutesBuffer = 5;
    const jiraCreatedIssuesQuery = `type != Epic AND (description is empty or description !~ "${descriptionLinearIdTag("")}") AND updated >= -${Math.ceil((Math.abs((new Date()).getTime() - checkpoint.getTime()) / 1000) / 60) + jiraSearchMinutesBuffer}m AND project = "${jiraProject}" ORDER BY created DESC`;

    const linearTeamClient = await linearClient.team(linearTeam);
    const [linearTeamId, linearProjects, linearTeamStates] = await Promise.all([
        (await linearClient.teams({ filter: { key: { eq: linearTeam } } })).nodes.find(_ => true)?.id,
        linearClient.projects().then(p => loadAllPagedNodes(p)),
        linearClient.workflowStates({ filter: { team: { key: { eq: linearTeam } } } }).then(loadAllPagedNodes),
    ])

    if (!linearTeamId) {
        throw `Linear Team ${linearTeam} not found`;
    }

    await Promise.all((await jiraClient.issueSearch.searchForIssuesUsingJql({
        jql: jiraCreatedIssuesQuery,
    })).issues!.map(async (i) => {
        let jiraUpdatedAt = new Date(i.fields.updated || i.fields.created || 0);
        if (jiraUpdatedAt < checkpoint) {
            console.log(`Skip syncing Jira issue ${i.key}, already synced`);
            return
        }
        if (jiraUpdatedAt > until) {
            console.log(`Skip syncing Jira issue ${i.key}: too fresh, skipping because Linear sync likely failed. Waiting for Linear to sync first.`);
            return
        }
        console.log(`Syncing issue created in Jira: ${i.key}`);

        let transitions = (await jiraClient.issues.getTransitions({ issueIdOrKey: i.key })).transitions?.map(t => t.name);
        let invalidTransition = transitions?.filter(t => !linearTeamStates.some(s => s.name === jiraToLinearStateName(t || "")));
        if ((invalidTransition?.length || 0) > 0) {
            console.log(`Skipping Jira ticket ${i.key}, unknown transitions: ${invalidTransition?.join(", ")}`)
            return
        }

        let inlineCardToUrl = (doc?: Omit<Version3Models.Document, 'version'>) => {
            if (!doc) {
                return
            }
            if (doc.type === "inlineCard" && doc.attrs?.url) {
                doc.type = "text"
                doc.text = doc.attrs?.url
                doc.marks = [{
                    type: "link",
                    attrs: {
                        href: doc.attrs?.url,
                    }
                }]
                delete doc.attrs
            }
            if (doc.type === "mediaSingle" && doc.content?.[0]?.attrs?.url) {
                doc.type = "paragraph"
                doc.content = [{
                    type: 'text', text: '!', // prefix the link with `!` to turn into picture link
                }, {
                    type: "text",
                    text: doc.content?.[0]?.attrs?.url,
                    marks: [{
                        type: "link",
                        attrs: {
                            href: doc.content?.[0]?.attrs?.url,
                        }
                    }]
                }]
                delete doc.attrs
            }
            if (doc.content) {
                doc.content.forEach(c => inlineCardToUrl(c))
            }
            return
        }
        inlineCardToUrl(i.fields.description)
        // TODO: replace: Not implemented as time of writing
        // const description = markdownTransformer.encode(jsonTransformer.parse(i.fields.description as JSONDocNode));
        let description: string = i.fields.description ? adf2md.convert(i.fields.description).result : "";
        // Force update
        if (description === "") {
            description = " ";
        }

        let linearIssue = await linearTeamClient.issues({
            filter: {
                searchableContent: {
                    contains: descriptionJiraIdTag(i.id),
                }
            }
        }).then(r => r.nodes.find(_ => true));

        if (!linearIssue) {
            linearIssue = (await linearTeamClient.issues({
                filter: {
                    attachments: {
                        url: {
                            eq: jiraIssueUrl(i.key),
                        },
                    }
                }
            })).nodes.find(_ => true);
        }

        let projectLink = i.fields.customfield_10013 ? await jiraClient.issueRemoteLinks.getRemoteIssueLinks({
            issueIdOrKey: i.fields.customfield_10013,
        }).then(links => (links as Version3Models.RemoteIssueLink[]).find(l => l.object?.title === LINEAR_LINK_TITLE)?.object?.url) : undefined;
        const linearIssueFields = {
            teamId: linearTeamId,
            title: i.fields.summary,
            description: description,
            assigneeId: linearUsers.find(u => u.email === i.fields.assignee?.emailAddress)?.id,
            projectId: linearProjects.find(p => p.url === projectLink)?.id,
            stateId: linearTeamStates.find(state => state.name === jiraToLinearStateName(i.fields.status.name!, i.fields.resolution?.name))?.id,
        }
        if (!linearIssue) {
            linearIssue = (await linearClient.createIssue({
                ...linearIssueFields,
                description: `${description}\n${descriptionJiraIdTag(i.id)}`,
            }).then(p => p.issue))!;
        }

        const linearIssueLinkedJira = await linearIssue?.jiraKey();
        if (!linearIssueLinkedJira) {
            await linearIssue?.createLinearLink(i.key);
        }

        // Update comments
        let jiraComments = await jiraClient.issueComments.getComments({ issueIdOrKey: i.key })
            .then(r => r.comments?.filter(c => c.author?.emailAddress !== env.JIRA_USER_EMAIL && new Date(c.updated || c.created || 0) > checkpoint))
            .then(comments => comments?.sort((a, b) => new Date(a.created || a.updated || 0).getTime() - new Date(b.created || b.updated || 0).getTime()));
        if (jiraComments) {
            let linearComments = await linearIssue?.comments({
                filter: {
                    user: {
                        isMe: {
                            eq: true,
                        }
                    }
                }
            }).then(r => loadAllPagedNodes(r));
            for (let jiraComment of jiraComments) {
                let linearCommentBody = `${jiraComment.author?.displayName} commented on Jira:\n\n${adf2md.convert(jiraComment.body).result}\n\n${descriptionJiraIdTag(jiraComment.id!)}`;
                let existingLinearComment = linearComments.find(c => c.body?.includes(descriptionJiraIdTag(jiraComment.id!)));
                if (existingLinearComment) {
                    await existingLinearComment.update({
                        body: linearCommentBody,
                    })
                } else {
                    await linearClient.createComment({
                        body: linearCommentBody,
                        issueId: linearIssue?.id,
                    });
                }
            }
        }

        // Update Linear ticket if needed
        let linearUpdatedAt = (linearIssue?.updatedAt || linearIssue?.createdAt || new Date(0));

        // Markdown conversion is inconsistent so let's try to approximate that ticket description is unchanged by comparing only word characters
        let withNormalizedDescription = ({ description, ...props }: { description?: string } & { [x: string]: any }) => ({
            ...props,
            description: description?.replace(/[^a-zA-Z0-9]/g, ""),
        })
        let linearIssueMatchable = withNormalizedDescription({
            ...linearIssue,
            teamId: linearIssue["_team"]?.id,
            projectId: linearIssue["_project"]?.id,
            stateId: linearIssue["_state"]?.id,
            assigneeId: linearIssue["_assignee"]?.id,
        });
        if (_.isMatch(linearIssueMatchable, withNormalizedDescription(linearIssueFields))) {
            console.log(`Skipping Linear ticket update ${linearIssue?.identifier} - already up-to-date`);
            return
        }

        console.log(`Syncing Jira to Linear - update times: Linear ${linearUpdatedAt} --- Jira ${jiraUpdatedAt}`)
        if (jiraUpdatedAt > linearUpdatedAt || linearIssue.description?.endsWith(descriptionJiraIdTag(i.id))) {
            console.log(`Updating Linear ticket ${linearIssue?.identifier}`);
            await linearIssue?.update(linearIssueFields);
        } else {
            console.log(`Skipping Linear ticket update ${linearIssue?.identifier} - Linear ticket is newer`);
            return
        }
    }));

}

async function main() {
    let checkpointNew = new Date();
    const lastAttemptExists = existsSync(lastAttemptFile);
    if (lastAttemptExists) {
        checkpointNew = new Date(readFileSync(lastAttemptFile, 'utf8'));
    } else {
        writeFileSync(lastAttemptFile, checkpointNew.toISOString())
    }

    const checkpointExists = existsSync(checkpointFile);
    const checkpointLast = checkpointExists ? new Date(readFileSync(checkpointFile, 'utf8')) : new Date(0);
    console.log(`Syncing since last checkpoint ${checkpointLast.toISOString()}`)

    const linearUsers = await linearClient.users().then(u => loadAllPagedNodes(u))
    await syncJiraIssuesToLinear(checkpointLast, checkpointNew, linearUsers);

    let jiraUsers = (await jiraClient.userSearch.findAssignableUsers({
        project: jiraProject,
        maxResults: 1000,
    }));
    const linearTeamClient = await linearClient.team(linearTeam);
    await syncLinearEntities(jiraUsers, linearUsers, (v) => linearTeamClient.projects(v), checkpointLast);
    await syncLinearEntities(jiraUsers, linearUsers, (v) => linearTeamClient.issues(v), checkpointLast);

    await syncLinearRelations(
        await loadAllPagedNodes(
            await linearClient.issueRelations({
                orderBy: LinearDocument.PaginationOrderBy.UpdatedAt,
            }))
    );

    writeFileSync(checkpointFile, checkpointNew.toISOString())
    unlinkSync(lastAttemptFile)
}

main();
