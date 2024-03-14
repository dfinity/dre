import os
import pathlib
import subprocess
import tempfile

from dotenv import load_dotenv
from release_index import Release, Version


class GitRepo:
    def __init__(self, repo, repo_cache_dir=pathlib.Path.home() / ".cache/git", main_branch="main"):
        self.repo = repo
        self.main_branch = main_branch
        self.dir = repo_cache_dir / "{}".format("/".join(repo.split("/")[-2:]))

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
        subprocess.check_call(
            [
                "git",
                "tag",
                f"release-{release.rc_name.removeprefix("rc--")}-{v.name}",
                v.version,
                "-f",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            cwd=repo.dir,
        )
    subprocess.check_call(
        [
            "git",
            "push",
            "origin",
            "--tags",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=repo.dir,
    )

def main():
    load_dotenv()

    repo = GitRepo(f"https://oauth2:{os.environ["GITHUB_TOKEN"]}@github.com/dfinity/ic-dre-testing.git", main_branch="master")
    push_release_tags(repo, Release(rc_name="rc--2024-02-28_23-01", versions=[
        Version(name="default", version="48da85ee6c03e8c15f3e90b21bf9ccae7b753ee6", release_notes_ready=True),
        # Version(name="p2p", version="a2cf671f832c36c0153d4960148d3e676659a747", release_notes_ready=True),
    ]))



if __name__ == "__main__":
    main()
