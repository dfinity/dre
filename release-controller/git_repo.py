import os
import pathlib
import subprocess
import tempfile

from dotenv import load_dotenv
from release_index import Release
from release_index import Version


class Commit:
    """Class for representing a git commit."""

    def __init__(
        self, sha: str, message: str, author: str, date: str, branches: list[str] = []
    ):  # pylint: disable=dangerous-default-value
        """Create a new Commit object."""
        self.sha = sha
        self.message = message
        self.author = author
        self.date = date
        self.branches = branches


class GitRepo:
    """Class for interacting with a git repository."""

    def __init__(self, repo: str, repo_cache_dir=pathlib.Path.home() / ".cache/git", main_branch="main"):
        """Create a new GitRepo object."""
        if not repo.startswith("https://"):
            raise ValueError("invalid repo")

        self.repo = repo
        self.main_branch = main_branch

        if not repo_cache_dir:
            self.cache_temp_dir = tempfile.TemporaryDirectory()
            repo_cache_dir = pathlib.Path(self.cache_temp_dir.name)

        self.dir = repo_cache_dir / (repo.split("@", 1)[1] if "@" in repo else repo.removeprefix("https://"))
        self.cache = {}

    def __del__(self):
        """Clean up the temporary directory."""
        if hasattr(self, "cache_temp_dir"):
            self.cache_temp_dir.cleanup()

    def ensure_branches(self, branches: list[str]):
        """Ensure that the given branches exist."""
        for branch in branches:
            try:
                subprocess.check_call(
                    ["git", "checkout", branch],
                    cwd=self.dir,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            except subprocess.CalledProcessError:
                print("Branch {} does not exist".format(branch))

        subprocess.check_call(
            ["git", "checkout", self.main_branch],
            cwd=self.dir,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    def show(self, obj: str) -> Commit | None:
        """Show the commit for the given object."""
        if obj in self.cache:
            return self.cache[obj]

        try:
            result = subprocess.run(
                [
                    "git",
                    "show",
                    "--no-patch",
                    "--format=%H%n%B%n%an%n%ad",
                    obj,
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=True,
                cwd=self.dir,
            )

            output = result.stdout.strip().splitlines()

            commit = Commit(output[0], output[1], output[2], output[3])
        except subprocess.CalledProcessError:
            return None

        try:
            branch_result = subprocess.run(
                ["git", "branch", "--contains", commit.sha],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=True,
                cwd=self.dir,
            )

            # Parse the result of the git branch command
            branches = branch_result.stdout.strip().splitlines()
            for branch in branches:
                branch = branch.strip()
                if branch.startswith("* "):
                    branch = branch[2:]
                if "remotes/origin/HEAD" in branch:
                    continue
                if branch.startswith("remotes/origin/"):
                    branch = branch[len("remotes/origin/") :]
                commit.branches.append(branch)

        except subprocess.CalledProcessError:
            return None

        self.cache[obj] = commit

        return commit

    def fetch(self):
        """Fetch the repository."""
        if (self.dir / ".git").exists():
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

    def is_ancestor(self, maybe_ancestor_commit: str, descendant_commit: str) -> bool:
        try:
            return 0 == subprocess.check_call(
                [
                    "git",
                    "merge-base",
                    "--is-ancestor",
                    maybe_ancestor_commit,
                    descendant_commit,
                ],
                cwd=self.dir,
            )
        except subprocess.CalledProcessError as e:
            return e.returncode == 0


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
        date = release.rc_name.removeprefix("rc--")
        tag = f"release-{date}-{v.name}"
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
        if (
            not subprocess.check_output(
                [
                    "git",
                    "ls-remote",
                    "origin",
                    f"refs/tags/{tag}",
                ],
                cwd=repo.dir,
            )
            .decode("utf-8")
            .strip()
        ):
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

    token = os.environ["GITHUB_TOKEN"]
    repo = GitRepo(f"https://oauth2:{token}@github.com/dfinity/ic-dre-testing.git", main_branch="master")
    push_release_tags(
        repo,
        Release(
            rc_name="rc--2024-02-21_23-01",
            versions=[
                Version(name="default", version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"),
                # Version(name="p2p", version="a2cf671f832c36c0153d4960148d3e676659a747"),
            ],
        ),
    )


if __name__ == "__main__":
    main()
