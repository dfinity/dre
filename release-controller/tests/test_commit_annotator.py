import pathlib
import tempfile
from commit_annotator import target_determinator
from git_repo import GitRepo


def _test_guestos_changed(object: str, changed: bool):
    with tempfile.TemporaryDirectory() as d:
        ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master", repo_cache_dir=pathlib.Path(d))
        assert target_determinator(object=object, ic_repo=ic_repo) == changed


def test_guestos_changed__not_guestos_change():
    _test_guestos_changed(object="00dc67f8d", changed=False)


def test_guestos_changed__bumped_dependencies():
    _test_guestos_changed(object="2d0835bba", changed=True)


def test_guestos_changed__github_dir_changed():
    _test_guestos_changed(object="94fd38099", changed=False)


def test_guestos_changed__replica_changed():
    _test_guestos_changed(object="951e895c7", changed=True)


def test_guestos_changed__cargo_lock_paths_only():
    _test_guestos_changed(object="5a250cb34", changed=False)
