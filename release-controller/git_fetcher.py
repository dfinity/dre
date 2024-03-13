import os
import pathlib
import subprocess
from release_index import Release


class GitFetcher:
    def __init__(self, repo, repo_cache_dir=pathlib.Path.home() / ".cache/git"):
        self.repo = repo
        self.dir = repo_cache_dir / "{}".format("/".join(repo.split("/")[-2:]))

    def fetch(self):
        if self.dir.exists():
            subprocess.check_call(
                ["git", "fetch"],
                cwd=self.dir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            subprocess.check_call(
                ["git", "reset", "--hard", "origin/main"],
                cwd=self.dir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        else:
            os.makedirs(self.dir)
            subprocess.check_call(
                [
                    "git",
                    "clone",
                    "https://github.com/dfinity/dre.git",
                    self.dir,
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

# TODO: test
def push_release_tag(git: GitFetcher, release: Release):
    for v in release.versions:
        subprocess.check_call(
            [
                "git",
                "tag",
                f"release-{release.rc_name.removeprefix("rc--")}-{v.name}",
                v.version,
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            cwd=git.dir,
        )
    subprocess.check_call(
        [
            "git",
            "push",
            "origin"
            "--tags",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=git.dir,
    )
