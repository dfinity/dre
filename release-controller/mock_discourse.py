from pydiscourse import DiscourseClient


class DiscourseClientMock(DiscourseClient):
    """A mock Discourse client."""

    def __init__(self):
        """Create a new mock client. The actual host needs to be mocked in the test."""
        self.host = "http://localhost:55555/"
        self.created_topics = []
        self.created_posts = []
        self.api_username = "test"

    def categories(self):
        """Return a list of categories."""
        return [
            {"id": i} | t
            for i, t in enumerate(
                [
                    {
                        "id": 5,
                        "name": "Governance",
                    }
                ]
            )
        ]

    def topics_by(self, _: str):
        """Return a list of topics."""
        return [{"id": i + 1} | t for i, t in enumerate(self.created_topics)]

    def topic_posts(self, topic_id: str):
        """Return a list of posts in a topic."""
        return {
            "post_stream": {
                "posts": [
                    p
                    for p in [{"id": i + 1} | p for i, p in enumerate(self.created_posts)]
                    if p["topic_id"] == topic_id
                ]
            }
        }

    def create_post(
        self,
        content,
        category_id=None,
        topic_id=None,
        title=None,
        tags=None,
        **kwargs,  # pylint: disable=unused-argument
    ):
        """Create a new post. If topic_id is not provided, a new topic is created."""
        if not topic_id:
            self.created_topics.append({"title": title, "category_id": category_id, "tags": tags or []})
            topic_id = self.topics_by("")[-1]["id"]
        self.created_posts.append(
            {
                "raw": content,
                "topic_id": topic_id,
                "yours": True,
                "can_edit": True,
            }
        )
        return self.topic_posts(topic_id=topic_id)["post_stream"]["posts"][-1]

    def update_post(self, post_id, content, edit_reason="", **kwargs):  # pylint: disable=unused-argument
        """Update an existing post."""
        self.created_posts[post_id - 1]["raw"] = content
