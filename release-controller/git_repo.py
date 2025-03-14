import contextlib
import logging
import os
import collections.abc
import pathlib
import subprocess
import tempfile
import typing

from dotenv import load_dotenv
from release_index import Release
from release_index import Version
from util import version_name


_LOGGER = logging.getLogger(__name__)


class FileChange(typing.TypedDict):
    file_path: str
    num_changes: int


def check_output(cmd: list[str], **kwargs: typing.Any) -> str:
    # _LOGGER.warning("CMD: %s", cmd)
    kwargs = kwargs or {}
    if "text" not in kwargs:
        kwargs["text"] = True
    return typing.cast(str, subprocess.check_output(cmd, **kwargs))


def check_call(cmd: list[str], **kwargs: typing.Any) -> int:
    # _LOGGER.warning("CMD: %s", cmd)
    return subprocess.check_call(cmd, **kwargs)


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


class GitRepoBehavior(typing.TypedDict):
    push_annotations: bool
    save_annotations: bool
    fetch_annotations: bool


class GitRepoAnnotator(object):
    def __init__(
        self,
        repo: "GitRepo",
        namespaces: collections.abc.Container[str],
    ):
        self.repo = repo
        self.namespaces = namespaces
        self.cache: dict[str, str | None] = {}
        md = self.dir / ".git" / "temp-notes"
        os.makedirs(md, exist_ok=True)
        self.td = tempfile.TemporaryDirectory(dir=md)
        self.changed = False

    def add(self, object: str, namespace: str, content: str) -> None:
        if namespace not in self.namespaces:
            raise ValueError(
                "cannot add note for %r with annotator limited to namespaces %s"
                % (namespace, self.namespaces)
            )
        cachekey = f"{namespace}-{object}".replace("/", "_").replace(".", "_")
        f = os.path.join(self.td.name, cachekey)
        if self.repo.behavior["save_annotations"]:
            with open(f, "w") as fh:
                fh.write(content)
            self.repo._notes(namespace, "add", f"--file={f}", object, "-f")
        self.cache[cachekey] = content
        self.changed = True

    def get(self, object: str, namespace: str) -> typing.Optional[str]:
        if namespace not in self.namespaces:
            raise ValueError(
                "cannot add note for %r with annotator limited to namespaces %s"
                % (namespace, self.namespaces)
            )
        cachekey = f"{namespace}-{object}"
        if cachekey not in self.cache:
            try:
                self.cache[cachekey] = self.repo._notes(namespace, "show", object)
            except subprocess.CalledProcessError:
                # It's not there!
                self.cache[cachekey] = None
        return self.cache[cachekey]

    def checkout(self, object: str) -> None:
        return self.repo.checkout(object)

    def parent(self, object: str) -> str:
        return self.repo.parent(object)

    @property
    def dir(self) -> pathlib.Path:
        return self.repo.dir


class GitRepo:
    """Class for interacting with a git repository."""

    def __init__(
        self,
        repo: str,
        repo_cache_dir: pathlib.Path = pathlib.Path.home() / ".cache/git",
        main_branch: str = "main",
        behavior: GitRepoBehavior | None = None,
    ) -> None:
        """Create a new GitRepo object."""
        if not repo.startswith("https://"):
            raise ValueError("invalid repo")

        self.repo = repo
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
        self.behavior = (
            behavior
            if behavior
            else {
                "push_annotations": True,
                "save_annotations": True,
                "fetch_annotations": True,
            }
        )
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
            _LOGGER.info(
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
            _LOGGER.info("Cloning repository %s to %s", self.repo, self.dir)
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
        """Checkout the given ref."""
        check_call(
            ["git", "reset", "--hard", "--quiet"],
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

    def _fetch_notes(self) -> None:
        ref = "refs/notes/*"
        check_call(
            ["git", "fetch", "origin", f"{ref}:{ref}", "-f", "--prune", "--quiet"],
            cwd=self.dir,
        )

    def _push_notes(self, namespace: str) -> None:
        check_call(
            ["git", "push", "origin", f"refs/notes/{namespace}", "-f", "--quiet"],
            cwd=self.dir,
        )

    def _notes(self, namespace: str, *args: str, silence_stderr: bool = False) -> str:
        cmd = ["git", "notes", f"--ref={namespace}", *args]
        return check_output(
            cmd,
            cwd=self.dir,
            stderr=subprocess.DEVNULL if silence_stderr else None,
        )

    @contextlib.contextmanager
    def annotator(
        self, namespaces: list[str]
    ) -> typing.Generator[GitRepoAnnotator, None, None]:
        if self.behavior["fetch_annotations"]:
            self._fetch_notes()
        ann = GitRepoAnnotator(self, namespaces)
        yield ann
        if ann.changed:
            for namespace in namespaces:
                if self.behavior["push_annotations"]:
                    self._push_notes(namespace)

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
                )


def main() -> None:
    load_dotenv()

    token = os.environ["GITHUB_TOKEN"]
    repo = GitRepo(
        f"https://oauth2:{token}@github.com/dfinity/ic-dre-testing.git",
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
