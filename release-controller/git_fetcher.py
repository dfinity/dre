import os
import pathlib
import subprocess


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
