import logging
import os
import pathlib
import pytest
import tarfile
import tempfile
import typing

from commit_annotator import target_determinator
from git_repo import GitRepo, GitRepoAnnotator


_LOGGER = logging.getLogger()


# It would be fantastic if this clone could be a plain @pytest.fixture
# but, due to our use of the xdist pytest plugin, that provides
# no savings (each process clones its own IC repo, rather than reusing
# the fixtures throughout the session).  It would still not be useful
# to parallelize the different tests on the table above, since the
# GitRepo class is not thread-safe anyway.
# So what do we do here?  We look for a variable (present in Bazel
# tests) that signals a promise that the tarball named in the variable
# contains an up-to-date clone of the IC repo.  If we find it, we
# extract it prior to instantiating the IC repo clone.
@pytest.fixture(scope="session")
def annotator() -> typing.Generator[GitRepoAnnotator, None, None]:
    with tempfile.TemporaryDirectory() as d:
        cache_dir = pathlib.Path(d)
        if "IC_REPO_SEED_TAR" in os.environ:
            unpacked_dir = cache_dir / "github.com" / "dfinity" / "ic.git"
            _LOGGER.info("Unpacking seed tarball of IC repo to %s...", unpacked_dir)
            os.makedirs(unpacked_dir, exist_ok=True)
            with tarfile.open(os.environ["IC_REPO_SEED_TAR"]) as seed_repo:
                seed_repo.extractall(unpacked_dir, filter="tar")
        else:
            _LOGGER.info("Cloning IC repo under cache directory %s...", cache_dir)
        ic_repo = GitRepo(
            "https://github.com/dfinity/ic.git",
            main_branch="master",
            repo_cache_dir=cache_dir,
            behavior={
                "push_annotations": False,
                "save_annotations": True,
                "fetch_annotations": False,
            },
        )
        _LOGGER.info("Clone of IC repo finished.  Going into annotator context.")
        with ic_repo.annotator([]) as annotator:
            yield annotator


def _test_guestos_changed(
    ann: GitRepoAnnotator, object: str, expect_changed: bool
) -> None:
    assert target_determinator(parent_object=object, cwd=ann.dir) == expect_changed


def test_guestos_changed__not_guestos_change(annotator: GitRepoAnnotator) -> None:
    _test_guestos_changed(annotator, object="00dc67f8d", expect_changed=False)


def test_guestos_changed__bumped_dependencies(annotator: GitRepoAnnotator) -> None:
    _test_guestos_changed(annotator, object="2d0835bba", expect_changed=True)


def test_guestos_changed__github_dir_changed(annotator: GitRepoAnnotator) -> None:
    _test_guestos_changed(annotator, object="94fd38099", expect_changed=False)


def test_guestos_changed__replica_changed(annotator: GitRepoAnnotator) -> None:
    _test_guestos_changed(annotator, object="951e895c7", expect_changed=True)


def test_guestos_changed__cargo_lock_paths_only(annotator: GitRepoAnnotator) -> None:
    _test_guestos_changed(annotator, object="5a250cb34", expect_changed=False)
