import pytest
import tempfile
import pathlib
from reconciler import Reconciler, ReconcilerState
from mock_discourse import DiscourseClientMock
from mock_google_docs import ReleaseNotesClientMock
from forum import ReleaseCandidateForumClient
from release_index_loader import StaticReleaseLoader
from publish_notes import PublishNotesClient
from github import Github


class TestReconcilerState(ReconcilerState):
    def __init__(self):
        self.tempdir = tempfile.TemporaryDirectory()
        super().__init__(pathlib.Path(self.tempdir.name))

    def __del__(self):
        self.tempdir.cleanup()


def test_e2e_mock_new_release(mocker):
    """
    Test the workflow when a new release is added to the index
    """

    discourse_client = DiscourseClientMock()
    forum_client = ReleaseCandidateForumClient(discourse_client)
    notes_client = ReleaseNotesClientMock()
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    config = """\
rollout:
  stages: []

releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: rc--2024-02-21_23-01
        release_notes_ready: false
"""
    reconciler = Reconciler(
        forum_client=forum_client,
        notes_client=notes_client,
        loader=StaticReleaseLoader(config),
        publish_client=PublishNotesClient(repo),
        nns_url="",
        state=TestReconcilerState(),
    )
    mocker.patch.object(reconciler.publish_client, "ensure_published")

    assert not notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    assert reconciler.publish_client.ensure_published.call_count == 0

    reconciler.reconcile()

    created_changelog = notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert "TODO:" == created_changelog
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    assert reconciler.publish_client.ensure_published.call_count == 0

    config = """\
rollout:
  stages: []

releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: rc--2024-02-21_23-01
        release_notes_ready: true
"""
    reconciler.loader = StaticReleaseLoader(config)
    # TODO: mock modifying google docs contents

    reconciler.reconcile()

    reconciler.publish_client.ensure_published.assert_called_once_with(
        version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        changelog="TODO:",
    )
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []

    # Changelog merged into main
    mocker.patch.object(reconciler.publish_client, "ensure_published")
    mocker.patch.object(
        reconciler.governance_canister,
        "replica_version_proposals",
        return_value={"2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f": [{"id": 12345}]},
    )
    reconciler.loader = StaticReleaseLoader(config, changelogs={"2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f": "TODO:"})
    reconciler.reconcile()

    # TODO: change to not called
    # assert reconciler.publish_client.ensure_published.call_count == 0
    # TODO: posts should be created
    # TODO: governance canister should be called
    # TODO: ic-admin should be executed with certain arguments, also with forum link
    # TODO: forum post should have been updated with proposal linked
    assert len(discourse_client.created_posts) == 1
    assert len(discourse_client.created_topics) == 1


def test_unelect_versions():
    {
  "key": "blessed_replica_versions",
  "version": 41803,
  "value": {
    "blessed_version_ids": [
      "8d4b6898d878fa3db4028b316b78b469ed29f293",
      "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
      "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
      "48da85ee6c03e8c15f3e90b21bf9ccae7b753ee6",
      "a2cf671f832c36c0153d4960148d3e676659a747",
      "778d2bb870f858952ca9fbe69324f9864e3cf5e7",
      "fff20526e154f8b8d24373efd9b50f588d147e91"
    ]
  }
}

def test_do_not_create_release_notes_for_old_releases():
    """
    Test that when the new release is added to the index, reconciler doesn't
    """


def test_elect_version():
    """
    Test that when the new release is added to the index, reconciler doesn't
    """


def test_do_no_elect_version_for_old_retired_releases():
    """
    Test that when the new release notes are added to the repo, reconciler doesn't re-elect versions that have since been retired
    """


def test_forum_post_is_updated_after_electing_new_version():
    """
    Test that when the new release is proposed, reconciler updates forum post description
    """


def test_adding_hotfix_for_last_two_releases():
    """
    Test adding a hotfix version to both releases works properly
    """


def test_forum_updates_when_versions_are_ready_out_of_order():
    """
    Test that versions are elected in order. This is important because forum posts on the topic need to be ordered in the same way.
    """


# TODO: test git diff is generated from the last non-rejected release.
# scenario 1: initial release was rejected, hotfix was applied
# scenario 2: initial release was rejected, another release replaces it


def test_release_is_rejected():
    """
    If the release proposal is rejected, do not create the proposal again.
    Also need to make sure that next proposal calculates changelog correctly.
    """


def test_update_only_active_releases():
    """
    We don't need to update historic releases to prevent potential unexpected behaviour
    """
