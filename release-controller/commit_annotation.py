import collections
import logging
import pathlib
import subprocess
import typing
import urllib

from const import (
    OsKind,
    GUESTOS,
    HOSTOS,
)
from git_repo import GitRepo
from util import check_call, check_output_binary, repr_ellipsized


_LOGGER = logging.getLogger(__name__)


CHANGED_NOTES_NAMESPACES: dict[OsKind, str] = {
    GUESTOS: "guestos-changed",
    HOSTOS: "hostos-changed",
}
COMMIT_BELONGS: typing.Literal["True"] = "True"
COMMIT_DOES_NOT_BELONG: typing.Literal["False"] = "False"

CommitInclusionState = typing.Literal["True"] | typing.Literal["False"]


class NotReady(Exception):
    """Exception raised when a commit is not yet annotated."""

    pass


class GitRepoAnnotator(object):
    def __init__(
        self,
        repo: "GitRepo",
        namespaces: collections.abc.Iterable[str],
        save_annotations: bool,
    ):
        """
        Returns a new GitRepoAnnotator based on the provided GitRepo.

        Annotations are not fetched by default!  Caller must call .fetch().
        If annotations are changed, caller must also call .push().
        """
        self.repo = repo
        self.namespaces = namespaces
        self.changed = False
        self.save_annotations = save_annotations

    def add(self, object: str, namespace: str, content: str) -> None:
        if namespace not in self.namespaces:
            raise ValueError(
                "cannot add note for %r with annotator limited to namespaces %s"
                % (namespace, self.namespaces)
            )
        _LOGGER.debug(
            "Adding note for commit %s in namespace %s: %s",
            object,
            namespace,
            repr_ellipsized(content),
        )
        if self.save_annotations:
            subprocess.run(
                args=[
                    "git",
                    "notes",
                    f"--ref={namespace}",
                    "add",
                    "--file=/dev/stdin",
                    "-f",
                    object,
                ],
                text=True,
                check=True,
                input=content,
                cwd=self.dir,
                stderr=subprocess.DEVNULL,
            )
        self.changed = True

    def has(self, object: str, namespace: str) -> bool:
        try:
            self.get(object, namespace)
            return True
        except KeyError:
            return False

    def get(self, object: str, namespace: str) -> bytes:
        if namespace not in self.namespaces:
            raise ValueError(
                "cannot get note for %r with annotator limited to namespaces %s"
                % (namespace, self.namespaces)
            )
        try:
            cmd = ["git", "notes", f"--ref={namespace}", "show", object]
            return check_output_binary(
                cmd,
                cwd=self.dir,
                stderr=subprocess.DEVNULL,
            )
        except subprocess.CalledProcessError:
            # It's not there!
            raise KeyError((namespace, object))

    def checkout(self, object: str) -> None:
        return self.repo.checkout(object)

    def parent(self, object: str) -> str:
        return self.repo.parent(object)

    @property
    def dir(self) -> pathlib.Path:
        return self.repo.dir

    def fetch(self) -> None:
        ref = "refs/notes/*"
        check_call(
            ["git", "fetch", "origin", f"{ref}:{ref}", "-f", "--quiet"],
            cwd=self.dir,
        )

    def push(self) -> None:
        if self.changed:
            for namespace in self.namespaces:
                check_call(
                    [
                        "git",
                        "push",
                        "origin",
                        f"refs/notes/{namespace}",
                        "-f",
                        "--quiet",
                    ],
                    cwd=self.dir,
                )


# Signature for a protocol (object) that carries a callable
# commit_changes_artifact that, given a commit and an OS kind,
# can determine whether the commit has changed that OS.
# Such callable should return NotReady when a commit is not yet
# annotated.
class ChangeDeterminatorProtocol(typing.Protocol):
    def commit_changes_artifact(
        self,
        commit: str,
        os_kind: OsKind,
    ) -> CommitInclusionState: ...


class LocalCommitChangeDeterminator(object):
    """Retrieves annotations from a local Git repository."""

    def __init__(self, ic_repo: GitRepo):
        """
        Creates a new commit change determinator.

        Upon creation, the freshest notes are fetched.
        """
        self.annotator = GitRepoAnnotator(
            ic_repo,
            list(CHANGED_NOTES_NAMESPACES.values()),
            save_annotations=False,
        )
        self.annotator.fetch()

    def commit_changes_artifact(
        self, commit: str, os_kind: OsKind
    ) -> CommitInclusionState:
        """
        Check if the os_kind (artifact) changed in the specifed commit
        by querying the local repo for git notes populated and pushed
        by commit annotator.
        """
        namespace = CHANGED_NOTES_NAMESPACES[os_kind]
        try:
            changed = (
                self.annotator.get(namespace=namespace, object=commit)
                .decode("utf-8")
                .strip()
            )
        except KeyError:
            raise NotReady(
                f"Could not find {os_kind} label for commit {commit}. Check out commit annotator logs and runbook: https://dfinity.github.io/dre/release.html#missing-guestos-label."
            )
        assert changed in [
            COMMIT_BELONGS,
            COMMIT_DOES_NOT_BELONG,
        ], "Expected a specific CommitInclusionState, not %r" % changed
        return typing.cast(CommitInclusionState, changed)


class CommitAnnotatorClientCommitChangeDeterminator(object):
    """Retrieves annotations from a commit annotator API server."""

    def __init__(self, base_url: str):
        self.base_url = base_url

    def commit_changes_artifact(
        self, commit: str, os_kind: OsKind
    ) -> CommitInclusionState:
        """
        Check if the os_kind (artifact) changed in the specifed commit
        by querying the commit annotator for git notes.
        """
        namespace = CHANGED_NOTES_NAMESPACES[os_kind]
        url = (
            self.base_url.rstrip("/")
            + f"/api/v1/commit/{commit}/annotation/{namespace}"
        )
        try:
            with urllib.request.urlopen(url) as response:
                changed = response.read().decode("utf-8").strip()
        except urllib.error.HTTPError as he:
            if he.code == 404:
                raise NotReady(
                    f"Could not find {os_kind} label for commit {commit}. Check out commit annotator logs and runbook: https://dfinity.github.io/dre/release.html#missing-guestos-label."
                ) from he
            raise
        assert changed in [
            COMMIT_BELONGS,
            COMMIT_DOES_NOT_BELONG,
        ], "Expected a specific CommitInclusionState, not %r" % changed
        return typing.cast(CommitInclusionState, changed)
