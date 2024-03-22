import os
from dotenv import load_dotenv
from pydantic_yaml import parse_yaml_raw_as
from pydiscourse import DiscourseClient
from release_index import Release, Model
from typing import Callable
from util import version_name


def _post_template(changelog, version_name, proposal=None):
    if not proposal:
        return f"We're preparing [a new IC release](https://github.com/dfinity/ic/tree/{version_name}). The changelog will be announced soon."

    return f"""\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/{version_name}).
The NNS proposal is here: [IC NNS Proposal 128295](https://dashboard.internetcomputer.org/proposal/{proposal}).

Here is a summary of the changes since the last release:

{changelog}
"""

class ReleaseCandidateForumPost:
    def __init__(self, version_name: str, changelog: str | None, proposal: int | None):
        self.version_name = version_name
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

    def created_posts(self):
        topic_posts = self.client.topic_posts(topic_id=self.topic_id)
        if not topic_posts:
            raise RuntimeError("failed to list topic posts")

        return [p for p in topic_posts.get("post_stream", {}).get("posts", {}) if p["yours"]]

    def update(self, changelog: Callable[[str], str | None], proposal: Callable[[str], int | None]):
        posts = [
                ReleaseCandidateForumPost(
                    version_name=version_name(self.release.rc_name, v.name),
                    changelog=changelog(v.version),
                    proposal=proposal(v.version),
                )
                for v in self.release.versions
            ]

        created_posts = self.created_posts()
        for i, p in enumerate(posts):
            print(p.version_name)
            if i < len(created_posts):
                self.client.update_post(
                    post_id=created_posts[i]["id"],
                    content=_post_template(version_name=p.version_name, changelog=p.changelog, proposal=p.proposal),
                )
            else:
                self.client.create_post(
                    topic_id=self.topic_id,
                    content=_post_template(version_name=p.version_name, changelog=p.changelog, proposal=p.proposal),
                )

    def post_url(self, version: str):
        post_index = [ i for i, v in enumerate(self.release.versions) if v.version == version ][0]
        post = self.client.post_by_id(post_id=self.created_posts()[post_index]["id"])
        if not post:
            raise RuntimeError("failed to find post")

        return f"{self.client.host.removesuffix("/")}/t/{post['topic_slug']}/{post['topic_id']}/{post["post_number"]}"

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



def main():
    load_dotenv()

    discourse_client = DiscourseClient(
        host=os.environ["DISCOURSE_URL"],
        api_username=os.environ["DISCOURSE_USER"],
        api_key=os.environ["DISCOURSE_KEY"],
    )
    index = parse_yaml_raw_as(
        Model,
        """
rollout:
  stages: []

releases:
  - rc_name: rc--2024-03-13_23-05
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: default
      - version: 31e9076fb99dfc36eb27fb3a2edc68885e6163ac
        name: feat
      - version: db583db46f0894d35bcbcfdea452d93abdadd8a6
        name: feat-hotfix1
""",
    )
    forum_client = ReleaseCandidateForumClient(
        discourse_client,
    )

    topic = forum_client.get_or_create(index.root.releases[0])
    topic.update(lambda _: None, lambda _: None)

    print(topic.post_url(version="31e9076fb99dfc36eb27fb3a2edc68885e6163ac"))


if __name__ == "__main__":
    main()
