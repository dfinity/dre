import logging
import pathlib
import tempfile

from commit_annotator import target_determinator
from git_repo import GitRepo


_LOGGER = logging.getLogger()


def test_determinator_touchstones() -> None:
    table = [
        ("not a guestos change", "00dc67f8d", False),
        ("bumped dependencies", "2d0835bba", True),
        ("github dir changed", "94fd38099", False),
        ("replica changed", "951e895c7", True),
        ("cargo lock paths only", "5a250cb34", False),
    ]
    with tempfile.TemporaryDirectory() as d:
        _LOGGER.info("Cloning IC repo...")
        ic_repo = GitRepo(
            "https://github.com/dfinity/ic.git",
            main_branch="master",
            repo_cache_dir=pathlib.Path(d),
            behavior={
                "push_annotations": False,
                "save_annotations": True,
                "fetch_annotations": True,
            },
        )
        _LOGGER.info("Clone of IC repo finished.  Going into annotator context.")
        with ic_repo.annotator([]) as annotator:
            for explanation, object, expected_result in table:
                _LOGGER.info(f"Testing touchstone {explanation}...")
                actual_result = target_determinator(object=object, ic_repo=annotator)
                assert (
                    actual_result == expected_result
                ), f"While running touchstone {explanation} on commit {object}, result {actual_result} != expected {expected_result}"
