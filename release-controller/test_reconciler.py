import pathlib
import tempfile
from types import SimpleNamespace
from unittest.mock import Mock

import git_repo
import pytest
import release_index
import requests
from forum import ReleaseCandidateForumClient
from github import Github
from mock_discourse import DiscourseClientMock
from mock_google_docs import ReleaseNotesClientMock
from publish_notes import PublishNotesClient
from pydantic_yaml import parse_yaml_raw_as
from reconciler import oldest_active_release
from reconciler import Reconciler
from reconciler import ReconcilerState
from reconciler import version_package_checksum
from reconciler import versions_to_unelect
from release_index_loader import StaticReleaseLoader


class TestReconcilerState(ReconcilerState):
    """Reconciler state that uses a temporary directory for storage."""

    def __init__(self):
        """Create a new TestReconcilerState."""
        self.tempdir = tempfile.TemporaryDirectory()
        super().__init__(pathlib.Path(self.tempdir.name))

    def __del__(self):
        """Clean up the temporary directory."""
        self.tempdir.cleanup()


@pytest.mark.skip(reason="not finished")
def test_e2e_mock_new_release(mocker):
    """Test the workflow when a new release is added to the index."""
    discourse_client = DiscourseClientMock()
    forum_client = ReleaseCandidateForumClient(discourse_client)
    notes_client = ReleaseNotesClientMock()
    github_client = Github()
    mocker.patch.object(github_client, "get_repo")
    repo = github_client.get_repo("dfinity/non-existent-mock")
    ic_repo_mock = Mock()
    mocker.patch("git_repo.push_release_tags")
    config = """\
rollout:
  stages: []

releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: default
  - rc_name: rc--2024-02-07_23-01
    versions:
      - version: 8d4b6898d878fa3db4028b316b78b469ed29f293
        name: default
"""
    reconciler = Reconciler(
        forum_client=forum_client,
        notes_client=notes_client,
        loader=StaticReleaseLoader(config),
        publish_client=PublishNotesClient(repo),
        nns_url="",
        state=TestReconcilerState(),
        ic_repo=ic_repo_mock,
        ignore_releases=[""],
    )
    mocker.patch.object(reconciler.publish_client, "ensure_published")

    assert not notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    assert reconciler.publish_client.ensure_published.call_count == 0
    assert git_repo.push_release_tags.call_count == 0  # pylint: disable=no-member

    reconciler.reconcile()

    created_changelog = notes_client.markdown_file("2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f")
    assert "TODO:" == created_changelog
    assert discourse_client.created_posts == []
    assert discourse_client.created_topics == []
    assert reconciler.publish_client.ensure_published.call_count == 0
    git_repo.push_release_tags.assert_called_once_with(  # pylint: disable=no-member
        ic_repo_mock,
        release_index.Release(
            rc_name="rc--2024-02-21_23-01",
            versions=[
                release_index.Version(
                    version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
                    name="default",
                )
            ],
        ),
    )

    config = """\
rollout:
  stages: []

releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: default
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


def test_versions_to_unelect():
    index = parse_yaml_raw_as(
        release_index.Model,
        """
releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: default
  - rc_name: rc--2024-02-14_23-01
    versions:
      - version: 31e9076fb99dfc36eb27fb3a2edc68885e6163ac
        name: default
      - version: 799e8401952ae9188242585cb9d52e19a8296a71
        name: hotfix
  - rc_name: rc--2024-02-07_23-01
    versions:
      - version: db583db46f0894d35bcbcfdea452d93abdadd8a6
        name: default
""",
    )

    assert versions_to_unelect(
        index,
        active_versions=["2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"],
        elected_versions=[
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "31e9076fb99dfc36eb27fb3a2edc68885e6163ac",
            "799e8401952ae9188242585cb9d52e19a8296a71",
            "db583db46f0894d35bcbcfdea452d93abdadd8a6",
        ],
    ) == [
        "31e9076fb99dfc36eb27fb3a2edc68885e6163ac",
        "799e8401952ae9188242585cb9d52e19a8296a71",
        "db583db46f0894d35bcbcfdea452d93abdadd8a6",
    ]
    assert versions_to_unelect(
        index,
        active_versions=["31e9076fb99dfc36eb27fb3a2edc68885e6163ac", "799e8401952ae9188242585cb9d52e19a8296a71"],
        elected_versions=[
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "31e9076fb99dfc36eb27fb3a2edc68885e6163ac",
            "799e8401952ae9188242585cb9d52e19a8296a71",
            "db583db46f0894d35bcbcfdea452d93abdadd8a6",
        ],
    ) == [
        "db583db46f0894d35bcbcfdea452d93abdadd8a6",
    ]
    assert versions_to_unelect(
        index,
        active_versions=["799e8401952ae9188242585cb9d52e19a8296a71"],
        elected_versions=[
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "31e9076fb99dfc36eb27fb3a2edc68885e6163ac",
            "799e8401952ae9188242585cb9d52e19a8296a71",
            "db583db46f0894d35bcbcfdea452d93abdadd8a6",
        ],
    ) == [
        "db583db46f0894d35bcbcfdea452d93abdadd8a6",
    ]

    # version not in release index
    assert versions_to_unelect(
        index,
        active_versions=["799e8401952ae9188242585cb9d52e19a8296a71", "9979097df5672caa85c5eec9f9878453a9f2deae"],
        elected_versions=[
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "31e9076fb99dfc36eb27fb3a2edc68885e6163ac",
            "799e8401952ae9188242585cb9d52e19a8296a71",
            "db583db46f0894d35bcbcfdea452d93abdadd8a6",
            "9979097df5672caa85c5eec9f9878453a9f2deae",
        ],
    ) == [
        "db583db46f0894d35bcbcfdea452d93abdadd8a6",
    ]


def test_oldest_active_release():
    index = parse_yaml_raw_as(
        release_index.Model,
        """
releases:
  - rc_name: rc--2024-02-21_23-01
    versions:
      - version: 2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f
        name: default
  - rc_name: rc--2024-02-14_23-01
    versions:
      - version: 31e9076fb99dfc36eb27fb3a2edc68885e6163ac
        name: default
  - rc_name: rc--2024-02-07_23-01
    versions:
      - version: db583db46f0894d35bcbcfdea452d93abdadd8a6
        name: default
""",
    )

    assert (
        oldest_active_release(index, active_versions=["2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"]).rc_name
        == "rc--2024-02-21_23-01"
    )
    assert (
        oldest_active_release(
            index,
            active_versions=["2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f", "31e9076fb99dfc36eb27fb3a2edc68885e6163ac"],
        ).rc_name
        == "rc--2024-02-14_23-01"
    )
    assert (
        oldest_active_release(index, active_versions=["31e9076fb99dfc36eb27fb3a2edc68885e6163ac"]).rc_name
        == "rc--2024-02-14_23-01"
    )


def test_version_package_checksum(mocker):
    def mock_download_files(url: str, timeout: int = 10):  # pylint: disable=unused-argument
        content = ""
        if url.endswith("SHA256SUMS"):
            content = """\
556b26661590495016052a58d07886e8dcce48c77a5dfc458fbcc5f01a95b1b3 *update-img-test.tar.gz
ed1ff4e1db979b0c89cf333c09777488a0c50a3ba74c0f9491d6ba153a8dbfdb *update-img-test.tar.zst
9ca7002a723b932c3fb25293fc541e0b156170ec1e9a2c6a83c9733995051187 *update-img.tar.gz
dff2072e34071110234b0cb169705efc13284e4a99b7795ef1951af1fe7b41ac *update-img.tar.zst
"""
        elif url.endswith(".tar.gz"):
            content = "some bytes..."

        return SimpleNamespace(content=content.encode())

    mocker.patch("requests.get", new=Mock(side_effect=mock_download_files))
    assert version_package_checksum("notimporant") == "9ca7002a723b932c3fb25293fc541e0b156170ec1e9a2c6a83c9733995051187"
    assert requests.get.call_count == 3  # pylint: disable=no-member


def test_version_package_checksum_mismatch(mocker):
    def mock_download_files(url: str, timeout: int = 10):  # pylint: disable=unused-argument
        content = ""
        if url.endswith("SHA256SUMS"):
            content = """\
556b26661590495016052a58d07886e8dcce48c77a5dfc458fbcc5f01a95b1b3 *update-img-test.tar.gz
ed1ff4e1db979b0c89cf333c09777488a0c50a3ba74c0f9491d6ba153a8dbfdb *update-img-test.tar.zst
9ca7002a723b932c3fb25293fc541e0b156170ec1e9a2c6a83c9733995051187 *update-img.tar.gz
dff2072e34071110234b0cb169705efc13284e4a99b7795ef1951af1fe7b41ac *update-img.tar.zst
"""
        elif "dfinity.network" in url:
            content = "some bytes..."
        else:
            content = "some other bytes..."

        return SimpleNamespace(content=content.encode())

    mocker.patch("requests.get", new=Mock(side_effect=mock_download_files))

    with pytest.raises(Exception) as e:
        version_package_checksum("notimporant")
        assert requests.get.call_count == 3  # pylint: disable=no-member

    assert repr(e.value) == repr(RuntimeError("checksums do not match"))
