import contextlib
import logging
import os
import pathlib
import pytest
import subprocess
import tarfile
import typing

from filelock import FileLock
from git_repo import GitRepo, GitRepoAnnotator


_LOGGER = logging.getLogger(__name__)


def arbitrary_git_repo(
    repository_url_without_protocol: str,
    main_branch: str,
    worker_id: str,
    tmp_path_factory: typing.Any,
) -> GitRepo:
    """Creates a GitRepo safely for each worker without cloning the repo for each call."""
    if worker_id == "master":
        mytemp = tmp_path_factory.getbasetemp() / "master"
    else:
        mytemp = typing.cast(pathlib.Path, tmp_path_factory.getbasetemp())
    basetemp = mytemp.parent

    lock = FileLock(str(basetemp / "git-clone-lock"))
    with lock:
        common_dir = basetemp / repository_url_without_protocol
        if not common_dir.exists():
            tarball = (
                os.environ["IC_REPO_SEED_TAR"]
                if (
                    repository_url_without_protocol == "github.com/dfinity/ic.git"
                    and "IC_REPO_SEED_TAR" in os.environ
                )
                else os.environ["DRE_REPO_SEED_TAR"]
                if (
                    repository_url_without_protocol == "github.com/dfinity/dre.git"
                    and "DRE_REPO_SEED_TAR" in os.environ
                )
                else None
            )
            if tarball:
                os.makedirs(common_dir, exist_ok=True)
                _LOGGER.info(
                    "Unpacking seed tarball %s repo to %s...", tarball, common_dir
                )
                with tarfile.open(tarball) as seed_repo:
                    seed_repo.extractall(common_dir, filter="tar")
            else:
                _LOGGER.info("About to clone Git repository into %s...", common_dir)
                GitRepo(
                    f"https://{repository_url_without_protocol}",
                    main_branch=main_branch,
                    repo_cache_dir=basetemp,
                )
    worker_clone = mytemp / repository_url_without_protocol
    os.makedirs(worker_clone.parent, exist_ok=True)
    if not worker_clone.exists():
        _LOGGER.info("Copying %s to %s", common_dir, worker_clone)
        subprocess.check_call(
            ["cp", "-R", "--reflink=auto", str(common_dir), str(worker_clone)]
        )
    _LOGGER.info("Yielding GitRepo from %s", worker_clone)
    return GitRepo(
        f"https://{repository_url_without_protocol}",
        main_branch=main_branch,
        repo_cache_dir=mytemp,
        fetch=False,  # it's already fetched.
    )


@pytest.fixture(scope="session")
def ic_repo(
    worker_id: str, tmp_path_factory: typing.Any
) -> typing.Generator[GitRepo, None, None]:
    yield arbitrary_git_repo(
        "github.com/dfinity/ic.git", "master", worker_id, tmp_path_factory
    )


@pytest.fixture(scope="session")
def dre_repo(
    worker_id: str, tmp_path_factory: typing.Any
) -> typing.Generator[GitRepo, None, None]:
    yield arbitrary_git_repo(
        "github.com/dfinity/dre.git", "main", worker_id, tmp_path_factory
    )


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
def annotator(ic_repo: GitRepo) -> typing.Generator[GitRepoAnnotator, None, None]:
    _LOGGER.info("Yielding Git repo annotator from Git repo at %s", ic_repo.dir)
    yield GitRepoAnnotator(ic_repo, [], False)


@pytest.fixture(scope="session")
def lock(
    tmp_path_factory: typing.Any, worker_id: str
) -> typing.Generator[FileLock, None, None]:
    if worker_id == "master":
        mytemp = tmp_path_factory.getbasetemp() / "master"
    else:
        mytemp = typing.cast(pathlib.Path, tmp_path_factory.getbasetemp())
    base_temp = mytemp.parent
    lock_file = base_temp / "serial.lock"
    yield FileLock(lock_file=str(lock_file))
    with contextlib.suppress(OSError):
        os.remove(path=lock_file)


@pytest.fixture()
def serialize(lock: FileLock) -> typing.Generator[None, None, None]:
    with lock.acquire(poll_interval=0.1):
        yield
