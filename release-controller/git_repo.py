import logging
import os
import pathlib
import subprocess
import sys
import tempfile
import threading
import typing

from dotenv import load_dotenv
from release_index import Release
from release_index import Version
from util import version_name, check_output, check_call


_LOGGER = logging.getLogger(__name__)


class FileChange(typing.TypedDict):
    file_path: str
    num_changes: int


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

    def __init__(
        self,
        repo: str,
        repo_cache_dir: pathlib.Path = pathlib.Path.home() / ".cache/git",
        main_branch: str = "main",
        fetch: bool = True,
    ) -> None:
        """
        Create a new GitRepo object.

        The repository will be cloned into the cache directory if it does
        not exist, and then fetched to the latest content present on the remote.

        If `fetch` is false, the repo will not be fetched after instantiation.
        This is useful during hermetic tests.
        """
        if not repo.startswith("https://"):
            raise ValueError("invalid repo")

        self.repo = repo
        self.operation_lock = threading.Lock()
        self.main_branch = main_branch

        if not repo_cache_dir:
            self.cache_temp_dir = tempfile.TemporaryDirectory()
            repo_cache_dir = pathlib.Path(self.cache_temp_dir.name)

        self.dir = repo_cache_dir / (
            "authed/{}".format(repo.split("@", 1)[1])
            if "@" in repo
            else repo.removeprefix("https://")
        )
        self.cache: dict[str, Commit] = {}
        if fetch:
            self.fetch()

    def __del__(self) -> None:
        """Clean up the temporary directory."""
        if hasattr(self, "cache_temp_dir"):
            self.cache_temp_dir.cleanup()

    def ensure_branches(self, branches: list[str]) -> None:
        """Ensure that the given branches exist."""
        for branch in branches:
            try:
                check_call(
                    ["git", "checkout", "--quiet", branch],
                    cwd=self.dir,
                )
            except subprocess.CalledProcessError:
                print("Branch {} does not exist".format(branch))

        check_call(
            ["git", "checkout", "--quiet", self.main_branch],
            cwd=self.dir,
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

    def fetch(self) -> None:
        """Fetch the repository."""
        if (self.dir / ".git").exists():
            _LOGGER.debug(
                "Updating repository in %s to latest origin/%s",
                self.dir,
                self.main_branch,
            )
            check_call(
                ["git", "fetch", "--quiet"],
                cwd=self.dir,
            )
            check_call(
                ["git", "reset", "--hard", "--quiet", f"origin/{self.main_branch}"],
                cwd=self.dir,
            )
        else:
            _LOGGER.info("Cloning repository to %s", self.dir)
            os.makedirs(self.dir, exist_ok=True)
            check_call(
                [
                    "git",
                    "clone",
                    self.repo,
                    str(self.dir),
                ],
            )
        check_call(
            [
                "git",
                "fetch",
                "--all",
                "--quiet",
            ],
            cwd=self.dir,
        )

    def merge_base(self, commit_a: str, commit_b: str) -> str:
        return check_output(
            [
                "git",
                "merge-base",
                commit_a,
                commit_b,
            ],
            cwd=self.dir,
        ).strip()

    def distance(self, commit_a: str, commit_b: str) -> int:
        return int(
            check_output(
                [
                    "git",
                    "rev-list",
                    "--count",
                    f"{commit_a}..{commit_b}",
                ],
                cwd=self.dir,
            )
        )

    def get_commits_info(
        self,
        git_commit_format: str,
        first_commit: str,
        last_commit: str,
    ) -> list[str]:
        """Get the info of commits in the range [first_commit, last_commit]."""
        return (
            check_output(
                [
                    "git",
                    "log",
                    "--format={}".format(git_commit_format),
                    "--no-merges",
                    "{}..{}".format(first_commit, last_commit),
                ],
                cwd=self.dir,
            )
            .rstrip()
            .splitlines()
        )

    def get_commit_info(
        self,
        git_commit_format: str,
        first_commit: str,
    ) -> str:
        """Get the info of commit."""
        return check_output(
            [
                "git",
                "show",
                "-s",
                "--format={}".format(git_commit_format),
                first_commit,
            ],
            cwd=self.dir,
            stderr=subprocess.DEVNULL,
        )

    def file(self, path: str) -> pathlib.Path:
        """Get the file for the given path."""
        return self.dir / path

    def file_contents(self, commit: str, path: pathlib.Path) -> bytes:
        """
        Get the contents file for the given path at a specific revision.

        This does not require the repository to be checked out -- only
        that the commit ID is fetched and present.

        path should be relative to the root of the repository.

        Raises FileNotFoundError if the git show command cannot find the file
        at that specific revision.
        """
        try:
            return subprocess.run(
                ["git", "show", f"{commit}:{path}"],
                capture_output=True,
                check=True,
                cwd=self.dir,
            ).stdout
        except subprocess.CalledProcessError as e:
            if b"fatal:" in e.stderr and b"does not exist" in e.stderr:
                raise FileNotFoundError(path) from e
            if b"fatal:" in e.stderr and b"invalid object name" in e.stderr:
                raise FileNotFoundError(path) from e
            sys.stderr.buffer.write(e.stderr)
            raise

    def file_changes_for_commit(self, commit_hash: str) -> list[FileChange]:
        cmd = [
            "git",
            "diff",
            "--numstat",
            f"{commit_hash}^..{commit_hash}",
        ]
        diffstat_output = check_output(
            cmd,
            cwd=self.dir,
            stderr=subprocess.DEVNULL,
        ).strip()

        parts = diffstat_output.splitlines()
        changes = []
        for line in parts:
            file_path = line.split()[2].strip()
            additions = line.split()[0].strip()
            deletions = line.split()[1].strip()
            additions = additions if additions != "-" else "0"
            deletions = deletions if deletions != "-" else "0"

            chg: FileChange = {
                "file_path": file_path,
                "num_changes": int(additions) + int(deletions),
            }
            changes.append(chg)

        return changes

    def checkout(self, ref: str) -> None:
        """Checkout the given ref.  The workspace will be clean after this."""
        _LOGGER.debug("Checking out ref %r", ref)
        check_call(
            ["git", "reset", "--hard", "--quiet"],
            cwd=self.dir,
        )
        check_call(
            ["git", "clean", "-fxdq"],
            cwd=self.dir,
        )
        check_call(
            ["git", "checkout", "--quiet", ref],
            cwd=self.dir,
        )
        if check_output(
            ["git", "branch", "--show-current"],
            cwd=self.dir,
        ).strip():
            check_call(
                ["git", "reset", "--hard", "--quiet", f"origin/{ref}"],
                cwd=self.dir,
            )

    def parent(self, object: str) -> str:
        return check_output(
            ["git", "log", "--pretty=%P", "-n", "1", object],
            cwd=self.dir,
        ).strip()

    def branch_list(self, pattern: str) -> typing.List[str]:
        return [
            b.strip().removeprefix("origin/")
            for b in check_output(
                ["git", "branch", "-r", "--list", f"origin/{pattern}"],
                cwd=self.dir,
            ).splitlines()
        ]

    def latest_commit_for_file(self, file: str) -> str:
        return check_output(
            ["git", "log", "-n", "1", "--pretty=format:%H", "--", file],
            cwd=self.dir,
        ).strip()

    # TODO: test
    def push_release_tags(self, release: Release) -> None:
        self.fetch()
        for v in release.versions:
            check_call(
                [
                    "git",
                    "fetch",
                    "--quiet",
                    "origin",
                    f"{v.version}:refs/remotes/origin/{v.version}-commit",
                ],
                cwd=self.dir,
            )
            tag = version_name(release.rc_name, v.name)
            check_call(
                [
                    "git",
                    "tag",
                    tag,
                    v.version,
                    "-f",
                ],
                cwd=self.dir,
            )
            tag_version = (
                check_output(
                    [
                        "git",
                        "ls-remote",
                        "origin",
                        f"refs/tags/{tag}",
                    ],
                    cwd=self.dir,
                )
                .strip()
                .split(" ")[0]
            )
            if tag_version == v.version:
                _LOGGER.info(
                    "RC %s: tag %s already exists on origin", release.rc_name, tag
                )
            else:
                _LOGGER.info(
                    "RC %s: pushing tag %s to the origin", release.rc_name, tag
                )
                env = os.environ.copy()
                env["GIT_TERMINAL_PROMPT"] = "0"
                check_call(
                    [
                        "git",
                        "push",
                        "--quiet",
                        "origin",
                        tag,
                        "-f",
                    ],
                    cwd=self.dir,
                    env=env,
                )


def main() -> None:
    load_dotenv()

    token = os.environ["GITHUB_TOKEN"]
    repo = GitRepo(
        f"https://{token}@github.com/dfinity/ic.git",
        main_branch="master",
    )
    repo.push_release_tags(
        Release(
            rc_name="rc--2024-02-21_23-01",
            versions=[
                Version(
                    name="default", version="2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f"
                ),
                # Version(name="p2p", version="a2cf671f832c36c0153d4960148d3e676659a747"),
            ],
        ),
    )


if __name__ == "__main__":
    main()
