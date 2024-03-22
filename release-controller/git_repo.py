import os
import pathlib
import subprocess
import tempfile

from dotenv import load_dotenv
from release_index import Release, Version


class GitRepo:
    def __init__(self, repo: str, repo_cache_dir=pathlib.Path.home() / ".cache/git", main_branch="main"):
        if not repo.startswith("https://"):
            raise ValueError("invalid repo")

        self.repo = repo
        self.main_branch = main_branch

        if not repo_cache_dir:
            self.cache_temp_dir = tempfile.TemporaryDirectory()
            repo_cache_dir = pathlib.Path(self.cache_temp_dir.name)

        self.dir = repo_cache_dir / (repo.split("@", 1)[1] if "@" in repo else repo.removeprefix("https://"))

    def __del__(self):
        if hasattr(self, 'cache_temp_dir'):
            self.cache_temp_dir.cleanup()

    def fetch(self):
        if (self.dir / '.git').exists():
            subprocess.check_call(
                ["git", "fetch"],
                cwd=self.dir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            subprocess.check_call(
                ["git", "reset", "--hard", f"origin/{self.main_branch}"],
                cwd=self.dir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        else:
            os.makedirs(self.dir, exist_ok=True)
            subprocess.check_call(
                [
                    "git",
                    "clone",
                    self.repo,
                    self.dir,
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        subprocess.check_call(
            [
                "git",
                "fetch",
                "--all",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            cwd=self.dir,
        )


# TODO: test
def push_release_tags(repo: GitRepo, release: Release):
    repo.fetch()
    for v in release.versions:
        subprocess.check_call(
            [
                "git",
                "fetch",
                "origin",
                f"{v.version}:refs/remotes/origin/{v.version}-commit",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            cwd=repo.dir,
        )
        tag = f"release-{release.rc_name.removeprefix("rc--")}-{v.name}"
        subprocess.check_call(
            [
                "git",
                "tag",
                tag,
                v.version,
                "-f",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            cwd=repo.dir,
        )
        if not subprocess.check_output(
            [
                "git",
                "ls-remote",
                "origin",
                f"refs/tags/{tag}",
            ],
            cwd=repo.dir,
        ).decode("utf-8").strip():
            subprocess.check_call(
                [
                    "git",
                    "push",
                    "origin",
                    tag,
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                cwd=repo.dir,
            )

def main():
    load_dotenv()

    repo = GitRepo(f"https://oauth2:{os.environ["GITHUB_TOKEN"]}@github.com/dfinity/ic-dre-testing.git", main_branch="master")
    push_release_tags(repo, Release(rc_name="rc--2024-02-21_23-01", versions=[
        Version(name="default", version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"),
        # Version(name="p2p", version="a2cf671f832c36c0153d4960148d3e676659a747"),
    ]))



if __name__ == "__main__":
    main()
