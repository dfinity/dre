import pytest
from forum import ReleaseCandidateForumClient, ReleaseCandidateForumPost
from mock_discourse import DiscourseClientMock
from release_index import Release, Version


def test_create_release_notes_on_new_release():
    """
    Test that when the new release is added to the index, reconciler creates release notes for engineers to edit
    """

    discourse_client = DiscourseClientMock()
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    forum_client = ReleaseCandidateForumClient(discourse_client=discourse_client)
    post = forum_client.get_or_create(
        Release(
            rc_name="rc--2024-02-21_23-06",
            versions=[
                Version(name="default", release_notes_ready=True, version="test1"),
                Version(name="feat", release_notes_ready=True, version="test2"),
            ],
        )
    )

    def changelog(v: str):
        return f"release notes for version {v}..."

    def proposal(v: str):
        return int(v.removeprefix("test"))

    post.update(changelog=changelog, proposal=proposal)
    assert discourse_client.created_posts == [
        {
            "raw": """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/2024-02-21_23-06-default).
The NNS proposal is here: [IC NNS Proposal 128295](https://dashboard.internetcomputer.org/proposal/1).

Here is a summary of the changes since the last release:

release notes for version test1...
""",
            "yours": True,
            "topic_id": 1,
        },
        {
            "raw": """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/2024-02-21_23-06-feat).
The NNS proposal is here: [IC NNS Proposal 128295](https://dashboard.internetcomputer.org/proposal/2).

Here is a summary of the changes since the last release:

release notes for version test2...
""",
            "yours": True,
            "topic_id": 1,
        },
    ]

    assert discourse_client.created_topics == [
        {
            "category_id": 5,
            "tags": ["replica", "release"],
            "title": "Proposal to elect new release rc--2024-02-21_23-06",
        }
    ]
