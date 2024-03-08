import pytest
from forum import ReleaseCandidateForumClient, ReleaseCandidateForumPost
from mock_discourse import DiscourseClientMock


def test_create_release_notes_on_new_release():
    """
    Test that when the new release is added to the index, reconciler creates release notes for engineers to edit
    """

    discourse_client = DiscourseClientMock()
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    forum_client = ReleaseCandidateForumClient(discourse_client=discourse_client)
    post = forum_client.get_or_create("rc--2024-02-21_23-06")
    post.update(
        [
            ReleaseCandidateForumPost("first random verasdfsion blablabla345."),
            ReleaseCandidateForumPost("second random version blablabla."),
        ]
    )
    assert discourse_client.created_posts == [
        {"raw": "first random verasdfsion blablabla345.", "yours": True, "topic_id": 1},
        {"raw": "second random version blablabla.", "yours": True, "topic_id": 1},
    ]

    assert discourse_client.created_topics == [
        {
            "category_id": 5,
            "tags": ["replica", "release"],
            "title": "Proposal to elect new release rc--2024-02-21_23-06",
        }
    ]
