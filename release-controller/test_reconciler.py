import pytest
from reconciler import Reconciler
from mock_discourse import DiscourseClientMock
from mock_google_docs import ReleaseNotesClientMock
from forum import ReleaseCandidateForumClient
from release_index_loader import StaticReleaseLoader
from publish_notes import PublishNotesClient
from github import Github


def test_create_release_notes_on_new_release(mocker):
    """
    Test that when the new release is added to the index, reconciler creates release notes for engineers to edit
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
    )
    mocker.patch.object(reconciler.publish_client, "ensure_published")

    assert not notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    assert reconciler.publish_client.ensure_published.call_count == 0

    reconciler.reconcile()

    created = notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert "TODO:" == created
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

    # TODO: mock file is merged into main


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
