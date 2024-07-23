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
from reconciler import find_base_release, oldest_active_release
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


def test_find_base_release():
    ic_repo = git_repo.GitRepo(
        f"https://github.com/dfinity/ic.git", main_branch="master", repo_cache_dir=pathlib.Path("/tmp/reconciler-cache")
    )
    index = parse_yaml_raw_as(
        release_index.Model,
        """
releases:
  - rc_name: rc--2024-07-10_23-01
    versions:
      - name: base
        version: a3831c87440df4821b435050c8a8fcb3745d86f6
      - name: storage-layer-disabled
        version: 0d2b3965c813cd3a39ceedacd97fa2eee8760074
  - rc_name: rc--2024-07-03_23-01
    versions:
      - # Successful qualification pipeline: https://gitlab.com/dfinity-lab/core/release/-/pipelines/1360352158
        name: base
        version: e4eeb331f874576126ef1196b9cdfbc520766fbd
      - # Successful qualification pipeline: https://gitlab.com/dfinity-lab/core/release/-/pipelines/1360514977
        name: storage-layer-disabled
        version: 5849c6daf2037349bd36dcb6e26ce61c2c6570d0
      - # Successful qualification pipeline: https://github.com/dfinity-ops/release/actions/runs/9877005239/job/27277530675
        name: hotfix-https-outcalls
        version: 16fabfd24617be66e08e00abc7ba3136bbd80010
      - # Successful qualification pipeline: https://github.com/dfinity-ops/release/actions/runs/9880530622/job/27289909086
        name: hotfix-https-outcalls-with-lsmt
        version: 7dee90107a88b836fc72e78993913988f4f73ca2
  - rc_name: rc--2024-06-26_23-01
    versions:
      - # Successful qualification pipeline: https://gitlab.com/dfinity-lab/core/release/-/pipelines/1350685950
        name: base
        version: 2e269c77aa2f6b2353ddad6a4ac3d5ddcac196b1
      - # Successful qualification pipeline: https://gitlab.com/dfinity-lab/core/release/-/pipelines/1350889767
        name: storage-layer-disabled
        version: b6c3687fb3a03ca65fcd49f0aadc499367904c8b
  - rc_name: rc--2024-06-19_23-01
    versions:
      - name: base
        version: e3fca54d11e19dc7134e374d9f472c5929f755f9
      - name: storage-layer-disabled
        version: ae3c4f30f198eba9c5b113ec32fdec90713c24a0
      - name: cycle-hotfix
        version: 9c006a50d364edf1403ef50b24c3be39dba8a5f6
  - rc_name: rc--2024-06-12_23-01
    versions:
      - name: base
        version: 246d0ce0784d9990c06904809722ce5c2c816269
      - name: storage-layer-disabled
        version: 2dfe3a1864d1b9a6df462e9503adf351036e7965
      - name: cycle-hotfix
        version: 48c500d1501e4165fc183e508872a2ef13fd0bef
  - rc_name: rc--2024-06-05_23-01
    versions:
      - name: base
        version: d19fa446ab35780b2c6d8b82ea32d808cca558d5
      - name: storage-layer-disabled
        version: 08f32722df2f56f1e5c1e603fee0c87c40b77cba
  - rc_name: rc--2024-05-29_23-02
    versions:
      - name: base
        version: b9a0f18dd5d6019e3241f205de797bca0d9cc3f8
      - name: hotfix-nns
        version: 42284da596a2596361f305b8d6d6097b0f40e6d6
  - rc_name: rc--2024-05-22_23-01
    versions:
      - name: base
        version: ec35ebd252d4ffb151d2cfceba3a86c4fb87c6d6
  - rc_name: rc--2024-05-15_23-02
    versions:
      - name: base
        version: 5ba1412f9175d987661ae3c0d8dbd1ac3e092b7d
      - name: storage-layer
        version: b6b2ef469bb00d38b48b789cae91251f27011b82
  - rc_name: rc--2024-05-09_23-02
    versions:
      - name: base
        version: 2c4566b7b7af453167785504ba3c563e09f38504
      - name: storage-layer
        version: 9866a6f5cb43c54e3d87fa02a4eb80d0f159dddb
      - name: hotfix-tecdsa
        version: 30bf45e80e6b5c1660cd12c6b554d4f1e85a2d11
  - rc_name: rc--2024-05-01_23-01
    versions:
      - name: base
        version: bb76748d1d225c08d88037e99ca9a066f97de496
      - name: storage-layer
        version: f58424c4ba894ab8a12c8e223655d5d378fb1010
  - rc_name: rc--2024-04-24_23-01
    versions:
      - name: base
        version: 80e0363393ea26a36b77e8c75f7f183cb521f67f
      - name: storage-layer
        version: 5e285dcaf77db014ac85d6f96ff392fe461945f5
  - rc_name: rc--2024-04-17_23-01
    versions:
      - name: base
        version: abcea3eff0be52dc5328e71de98288991de854bf
      - name: query-stats
        version: 0a51fd74f08b2e6f23d6e1d60f1f52eb73b40ccc
      - name: hotfix-bitcoin
        version: 687de34189de20c5346e6b6167d22bcdd11e7ae5
      - name: hotfix-bitcoin-query-stats
        version: 63acf4f88b20ec0c6384f4e18f0f6f69fc5d9b9f
  - rc_name: rc--2024-04-10_23-01
    versions:
      - name: base
        version: 19dbb5cc6e3dc85c0ccd899b3182552612f1607d
      - name: query-stats
        version: 02dcaf3ccdfe46bd959d683d43c5513d37a1420d
      - name: hotfix-bitcoin
        version: 33dd2ef2184a64c00e64ff0412e7378d46507005
      - name: hotfix-bitcoin-query-stats
        version: 4e9b02fc3c0fa377b2fba44b15841d6ef73593a3
""",
    )

    assert find_base_release(ic_repo, index, "48c500d1501e4165fc183e508872a2ef13fd0bef") == (
        "246d0ce0784d9990c06904809722ce5c2c816269",
        "release-2024-06-12_23-01-base",
    )
    assert find_base_release(ic_repo, index, "246d0ce0784d9990c06904809722ce5c2c816269") == (
        "d19fa446ab35780b2c6d8b82ea32d808cca558d5",
        "release-2024-06-05_23-01-base",
    )
    assert find_base_release(ic_repo, index, "9866a6f5cb43c54e3d87fa02a4eb80d0f159dddb") == (
        "2c4566b7b7af453167785504ba3c563e09f38504",
        "release-2024-05-09_23-02-base",
    )
    assert find_base_release(ic_repo, index, "63acf4f88b20ec0c6384f4e18f0f6f69fc5d9b9f") == (
        "0a51fd74f08b2e6f23d6e1d60f1f52eb73b40ccc",
        "release-2024-04-17_23-01-query-stats",
    )
    assert find_base_release(ic_repo, index, "0d2b3965c813cd3a39ceedacd97fa2eee8760074") == (
        "a3831c87440df4821b435050c8a8fcb3745d86f6",
        "release-2024-07-10_23-01-base",
    )
