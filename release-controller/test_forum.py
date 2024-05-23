import httpretty.utils
from forum import ReleaseCandidateForumClient
from mock_discourse import DiscourseClientMock
from release_index import Release
from release_index import Version


@httpretty.activate(verbose=True, allow_net_connect=False)
def test_create_release_notes_on_new_release():
    """Release notes are created when a new release is added to the index."""
    discourse_client = DiscourseClientMock()
    get_url = discourse_client.host + "/posts/1.json"
    httpretty.register_uri(
        httpretty.GET,
        get_url,
        body='{"raw": "bogus text", "can_edit": true}',
        content_type="application/json; charset=utf-8",
    )
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    forum_client = ReleaseCandidateForumClient(discourse_client=discourse_client)
    post = forum_client.get_or_create(
        Release(
            rc_name="rc--2024-02-21_23-06",
            versions=[
                Version(name="default", version="test1"),
                Version(name="feat", version="test2"),
            ],
        )
    )

    def changelog(v: str):
        return f"release notes for version {v}..."

    def proposal(v: str):
        return int(v.removeprefix("test"))

    post.update(changelog=changelog, proposal=proposal)
    expected_post_1 = {
        "raw": """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/release-2024-02-21_23-06-default).
The NNS proposal is here: [IC NNS Proposal 1](https://dashboard.internetcomputer.org/proposal/1).

Here is a summary of the changes since the last release:

release notes for version test1...
""",
        "yours": True,
        "topic_id": 1,
        "can_edit": True,
    }
    expected_post_2 = {
        "raw": """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/release-2024-02-21_23-06-feat).
The NNS proposal is here: [IC NNS Proposal 2](https://dashboard.internetcomputer.org/proposal/2).

Here is a summary of the changes since the last release:

release notes for version test2...
""",
        "yours": True,
        "topic_id": 1,
        "can_edit": True,
    }
    assert discourse_client.created_posts == [expected_post_1, expected_post_2]

    assert discourse_client.created_topics == [
        {
            "category_id": 5,
            "tags": ["replica", "release"],
            "title": "Proposal to elect new release rc--2024-02-21_23-06",
        }
    ]
