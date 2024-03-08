from pydiscourse import DiscourseClient


class ReleaseCandidateForumPost:
    def __init__(self, release_notes: str):
        self.release_notes = release_notes


class ReleaseCandidateForumTopic:
    def __init__(self, release: str, client: DiscourseClient, governance_category):
        self.release = release
        self.client = client
        self.governance_category = governance_category
        topic = next((t for t in client.topics_by(self.client.api_username) if self.release in t["title"]), None)
        if topic:
            self.topic_id = topic["id"]
        else:
            post = client.create_post(
                category_id=governance_category["id"],
                content="The proposal for the next release will be announced soon.",
                tags=["replica", "release"],
                title="Proposal to elect new release {}".format(self.release),
            )
            self.topic_id = post["topic_id"]

    def update(self, posts: list[ReleaseCandidateForumPost]):
        created_posts = [
            p
            for p in self.client.topic_posts(topic_id=self.topic_id).get("post_stream", {}).get("posts", {})
            if p["yours"]
        ]
        for i, p in enumerate(posts):
            if i < len(created_posts):
                self.client.update_post(
                    post_id=created_posts[i]["id"],
                    content=p.release_notes,
                )
            else:
                self.client.create_post(
                    topic_id=self.topic_id,
                    content=p.release_notes,
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

    def get_or_create(self, release: str) -> ReleaseCandidateForumTopic:
        return ReleaseCandidateForumTopic(
            release, client=self.discourse_client, governance_category=self.governance_category
        )
