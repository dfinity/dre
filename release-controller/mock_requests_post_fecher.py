from forum import RequestsPostFetcher


class MockRequestsPostFetcher(RequestsPostFetcher):
    """A mock requests post fetcher client."""

    def __init__(self, created_posts):
        """Create mock post fetcher."""
        self.created_posts = created_posts

    def fetch_topics_posts(self, topic_id):
        """Fetch mocked topics."""
        return {
            "post_stream": {
                "posts": [
                    p
                    for p in [{"id": i + 1} | p for i, p in enumerate(self.created_posts)]
                    if p["topic_id"] == topic_id
                ]
            }
        }
