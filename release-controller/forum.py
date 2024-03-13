import itertools
from pydiscourse import DiscourseClient
from release_index import Release
from typing import Callable



def _post_template(changelog, release, proposal=None):
    if not proposal:
        return "We're preparing [a new IC release](https://github.com/dfinity/ic/tree/{release}). The changelog will be announced soon."

    return f"""\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/{release}).
The NNS proposal is here: [IC NNS Proposal 128295](https://dashboard.internetcomputer.org/proposal/{proposal}).

Here is a summary of the changes since the last release:

{changelog}
"""


def release_name(rc_name: str, name: str):
    return f"{rc_name.removeprefix("rc--")}-{name}"


class ReleaseCandidateForumPost:
    def __init__(self, release: str, changelog: str | None, proposal: int | None):
        self.release = release
        self.changelog = changelog
        self.proposal = proposal


class ReleaseCandidateForumTopic:
    def __init__(self, release: Release, client: DiscourseClient, governance_category):
        self.release = release
        self.client = client
        self.governance_category = governance_category
        topic = next((t for t in client.topics_by(self.client.api_username) if self.release.rc_name in t["title"]), None)
        if topic:
            self.topic_id = topic["id"]
        else:
            post = client.create_post(
                category_id=governance_category["id"],
                content="The proposal for the next release will be announced soon.",
                tags=["replica", "release"],
                title="Proposal to elect new release {}".format(self.release.rc_name),
            )
            if post:
                self.topic_id = post["topic_id"]
            else:
                raise RuntimeError("post not created")

    def update(self, changelog: Callable[[str], str | None], proposal: Callable[[str], int | None]):
        posts = [
                ReleaseCandidateForumPost(
                    release=release_name(self.release.rc_name, v.name),
                    changelog=changelog(v.version),
                    proposal=proposal(v.version),
                )
                for v in self.release.versions
            ]
        topic_posts = self.client.topic_posts(topic_id=self.topic_id)
        if not topic_posts:
            raise RuntimeError("failed to list topic posts")

        created_posts = [p for p in topic_posts.get("post_stream", {}).get("posts", {}) if p["yours"]]
        for i, p in enumerate(posts):
            if i < len(created_posts):
                self.client.update_post(
                    post_id=created_posts[i]["id"],
                    content=_post_template(release=p.release, changelog=p.changelog, proposal=p.proposal),
                )
            else:
                self.client.create_post(
                    topic_id=self.topic_id,
                    content=_post_template(release=p.release, changelog=p.changelog, proposal=p.proposal),
                )

    def add_version(self, content: str):
        self.client.create_post(
            topic_id=self.topic_id,
            content=content,
        )


class ReleaseCandidateForumClient:
    def __init__(self, discourse_client: DiscourseClient):
        self.discourse_client = discourse_client
        self.governance_category = next(c for c in self.discourse_client.categories() if c["name"] == "Governance")

    def get_or_create(self, release: Release) -> ReleaseCandidateForumTopic:
        return ReleaseCandidateForumTopic(
            release=release, client=self.discourse_client, governance_category=self.governance_category,
        )
