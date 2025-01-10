import logging
import os
from typing import Callable

from dotenv import load_dotenv
from pydiscourse import DiscourseClient
from release_index import Release
from util import version_name


def _post_template(changelog, version_name, proposal=None):
    if not proposal:
        return f"We're preparing [a new IC release](https://github.com/dfinity/ic/tree/{version_name}). The changelog will be announced soon."

    return f"""\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/{version_name}).
The NNS proposal is here: [IC NNS Proposal {proposal}](https://dashboard.internetcomputer.org/proposal/{proposal}).

Here is a summary of the changes since the last release:

{changelog}
"""


class ReleaseCandidateForumPost:
    """A post in a release candidate forum topic."""

    def __init__(self, version_name: str, changelog: str | None, proposal: int | None):
        """Create a new post."""
        self.version_name = version_name
        self.changelog = changelog
        self.proposal = proposal


class ReleaseCandidateForumTopic:
    """A topic in the governance category for a release candidate."""

    def __init__(
        self,
        release: Release,
        client: DiscourseClient,
        nns_proposal_discussions_category,
    ):
        """Create a new topic."""
        self.posts_count = 1
        self.release = release
        self.client = client
        self.nns_proposal_discussions_category = nns_proposal_discussions_category
        topic = next(
            (
                t
                for t in client.topics_by(self.client.api_username)
                if self.release.rc_name in t["title"]
            ),
            None,
        )
        if topic:
            self.topic_id = topic["id"]
            self.posts_count = topic["posts_count"]
        else:
            post = client.create_post(
                category_id=nns_proposal_discussions_category["id"],
                content="The proposal for the next release will be announced soon.",
                tags=["IC-OS-election", "release"],
                title="Proposal to elect new release {}".format(self.release.rc_name),
            )
            if post:
                self.topic_id = post["topic_id"]
            else:
                raise RuntimeError("post not created")

    def created_posts(self):
        """Return a list of posts created by the current user."""
        results = []
        for p in range((self.posts_count - 1) // 20 + 1):
            topic_posts = self.client._get(f"/t/{self.topic_id}.json", page=p + 1)
            if not topic_posts:
                raise RuntimeError("failed to list topic posts")
            results.extend(
                [
                    p
                    for p in topic_posts.get("post_stream", {}).get("posts", {})
                    if p["yours"]
                ]
            )
        return results

    def update(
        self,
        changelog: Callable[[str], str | None],
        proposal: Callable[[str], int | None],
    ):
        """Update the topic with the latest release information."""
        posts = [
            ReleaseCandidateForumPost(
                version_name=version_name(self.release.rc_name, v.name),
                changelog=changelog(v.version),
                proposal=proposal(v.version),
            )
            for v in self.release.versions if not v.security_fix
        ]

        created_posts = self.created_posts()
        for i, p in enumerate(posts):
            if i < len(created_posts):
                post_id = created_posts[i]["id"]
                content_expected = _post_template(
                    version_name=p.version_name,
                    changelog=p.changelog,
                    proposal=p.proposal,
                )
                post = self.client.post_by_id(post_id)
                if post["raw"] == content_expected:
                    # log the complete URL of the post
                    logging.info("post up to date: %s", self.post_to_url(post))
                    continue
                elif post["can_edit"]:
                    logging.info("updating post %s", post_id)
                    self.client.update_post(post_id=post_id, content=content_expected)
                else:
                    logging.warning("post is not editable %s", post_id)
            else:
                self.client.create_post(
                    topic_id=self.topic_id,
                    content=_post_template(
                        version_name=p.version_name,
                        changelog=p.changelog,
                        proposal=p.proposal,
                    ),
                )

    def post_url(self, version: str):
        """Return the URL of the post for the given version."""
        post_index = [
            i for i, v in enumerate(self.release.versions) if v.version == version
        ][0]
        post = self.client.post_by_id(post_id=self.created_posts()[post_index]["id"])
        if not post:
            raise RuntimeError("failed to find post")
        return self.post_to_url(post)

    def post_to_url(self, post: dict):
        """Return the complete URL of the given post."""
        host = self.client.host.removesuffix("/")
        return f"{host}/t/{post['topic_slug']}/{post['topic_id']}/{post['post_number']}"

    def add_version(self, content: str):
        """Add a new version to the topic."""
        self.client.create_post(
            topic_id=self.topic_id,
            content=content,
        )


class ReleaseCandidateForumClient:
    """A client for interacting with release candidate forum topics."""

    def __init__(self, discourse_client: DiscourseClient):
        """Create a new client."""
        self.discourse_client = discourse_client
        self.nns_proposal_discussions_category = next(
            (
                c
                for c in self.discourse_client.categories(include_subcategories="true")
                if c["name"] == "NNS proposal discussions"
            ),
            self.discourse_client.category(76)[
                "category"
            ],  # hardcoded category id, seems like "include_subcategories" is not working
        )

    def get_or_create(self, release: Release) -> ReleaseCandidateForumTopic:
        """Get or create a forum topic for the given release."""
        return ReleaseCandidateForumTopic(
            release=release,
            client=self.discourse_client,
            nns_proposal_discussions_category=self.nns_proposal_discussions_category,
        )


def main():
    load_dotenv()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    #     index = parse_yaml_raw_as(
    #         Model,
    #         """
    # rollout:
    #   stages: []

    # releases:
    #   - rc_name: rc--2024-03-13_23-05
    #     versions:
    #       - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
    #         name: default
    #       - version: 31e9076fb99dfc36eb27fb3a2edc68885e6163ac
    #         name: feat
    #       - version: db583db46f0894d35bcbcfdea452d93abdadd8a6
    #         name: feat-hotfix1
    # """,
    #     )
    forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )


#     topic = forum_client.get_or_create(index.root.releases[0])
#     topic.update(lambda _: None, lambda _: None)

# print(topic.post_url(version="31e9076fb99dfc36eb27fb3a2edc68885e6163ac"))


if __name__ == "__main__":
    main()
