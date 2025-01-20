# type: ignore

import httpretty.utils
from release_index import Release
from release_index import Version
import dryrun

import pytest


def test_create_release_notes_on_new_release() -> None:
    """Release notes are created when a new release is added to the index."""
    dc = dryrun.StubDiscourseClient()
    forum_client = dryrun.ForumClient(dc)
    post = forum_client.get_or_create(
        Release(
            rc_name="rc--2024-02-21_23-06",
            versions=[
                Version(name="default", version="test1"),
                Version(name="feat", version="test2"),
            ],
        )
    )

    def changelog(v: str, security_fix: bool):
        return f"release notes for version {v}{' (security fix)' if security_fix else ''}..."

    def proposal(v: str):
        return int(v.removeprefix("test"))

    post.update(summary_retriever=changelog, proposal_id_retriever=proposal)
    raw = """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/release-2024-02-21_23-06-default).
The NNS proposal is here: [IC NNS Proposal 1](https://dashboard.internetcomputer.org/proposal/1).

Here is a summary of the changes since the last release:

release notes for version test1...
"""
    expected_post_1 = {
        "raw": raw,
        "cooked": raw,
        "yours": True,
        "topic_id": 0,
        "topic_slug": "Proposal-to-elect-new-release-rc--2024-02-21_23-06",
        "can_edit": True,
        "id": 1000,
        "post_number": 1000,
        "reply_count": 0,
    }
    raw = """\
Hello there!

We are happy to announce that voting is now open for [a new IC release](https://github.com/dfinity/ic/tree/release-2024-02-21_23-06-feat).
The NNS proposal is here: [IC NNS Proposal 2](https://dashboard.internetcomputer.org/proposal/2).

Here is a summary of the changes since the last release:

release notes for version test2...
"""
    expected_post_2 = {
        "raw": raw,
        "cooked": raw,
        "yours": True,
        "topic_id": 0,
        "topic_slug": "Proposal-to-elect-new-release-rc--2024-02-21_23-06",
        "can_edit": True,
        "id": 1001,
        "post_number": 1001,
        "reply_count": 0,
    }
    assert dc.topics[0]["post_stream"]["posts"] == [expected_post_1, expected_post_2]

    assert dc.topics[0]["title"] == "Proposal to elect new release rc--2024-02-21_23-06"
    assert dc.topics[0]["posts_count"] == 2


@pytest.mark.skip("broken")
@httpretty.activate(verbose=True, allow_net_connect=False)
def test_create_post_in_new_category():
    """Release notes are created when a new release is added to the index."""
    dc = dryrun.StubDiscourseClient()
    forum_client = dryrun.ForumClient(dc)

    post = forum_client.get_or_create(
        Release(
            rc_name="rc--2024-02-21_23-06",
            versions=[
                Version(name="default", version="test1"),
                Version(name="feat", version="test2"),
            ],
        )
    )

    def changelog(v: str, security_fix: bool):
        return f"release notes for version {v}{' (security fix)' if security_fix else ''}..."

    def proposal(v: str):
        return int(v.removeprefix("test"))

    post.update(summary_retriever=changelog, proposal_id_retriever=proposal)

    post = forum_client.get_or_create(
        Release(
            rc_name="rc--2024-02-28_23-06",
            versions=[
                Version(name="default", version="test3"),
            ],
        )
    )
    post.update(summary_retriever=changelog, proposal_id_retriever=proposal)

    assert len(dc.topics) == 2
    assert len(dc.topics[0]["post_stream"]["posts"]) == 2
    assert len(dc.topics[1]["post_stream"]["posts"]) == 1
