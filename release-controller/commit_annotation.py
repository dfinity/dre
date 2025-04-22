import logging
import pathlib
import subprocess
import time
import typing
import urllib

from const import (
    OsKind,
    GUESTOS,
    HOSTOS,
)
from git_repo import GitRepoAnnotator, GitRepo
from tenacity import retry, stop_after_delay, retry_if_exception_type, after_log
from util import resolve_binary


_LOGGER = logging.getLogger()


BAZEL_TARGETS = {
    # All targets that produce the update image for GuestOS.
    GUESTOS: "deps(//ic-os/guestos/envs/prod:update-img.tar.zst)",
    # All targets that produce the HostOS disk image united with the targets
    # of the SetupOS disk image minus HostOS and GuestOS disk images.
    HOSTOS: """
    deps(
        //ic-os/hostos/envs/prod:disk-img.tar.zst
    ) union (
        deps(
            //ic-os/setupos/envs/prod:disk-img.tar.zst
        ) except deps(
            //ic-os/hostos/envs/prod:disk-img.tar.zst
        ) except deps(
            //ic-os/guestos/envs/prod:disk-img.tar.zst
        ) except //ic-os/setupos/envs/prod/... except //ic-os/setupos/envs/prod:guest-os.img.tar.zst
    )
    """,
}
CHANGED_NOTES_NAMESPACES: dict[OsKind, str] = {
    GUESTOS: "guestos-changed",
    HOSTOS: "hostos-changed",
}
COMMIT_BELONGS: typing.Literal["True"] = "True"
COMMIT_DOES_NOT_BELONG: typing.Literal["False"] = "False"
COMMIT_COULD_NOT_BE_ANNOTATED: typing.Literal["Failed"] = "Failed"

CommitInclusionState = (
    typing.Literal["True"] | typing.Literal["False"] | typing.Literal["Failed"]
)


class NotReady(Exception):
    """Exception raised when a commit is not yet annotated."""

    pass


# target-determinator sometimes fails on first few tries
# we will therefore blow up after 180 seconds
@retry(
    stop=stop_after_delay(180),
    retry=retry_if_exception_type(subprocess.CalledProcessError),
    after=after_log(_LOGGER, logging.ERROR),
)
def target_determinator(
    cwd: pathlib.Path, parent_object: str, bazel_targets: str
) -> str:
    logger = _LOGGER.getChild("target_determinator").getChild(parent_object)
    p = subprocess.run(
        [
            resolve_binary("target-determinator"),
            "-before-query-error-behavior=fatal",
            "-delete-cached-worktree",
            f"-bazel={resolve_binary("bazel")}",
            "--targets",
            bazel_targets,
            parent_object,
        ],
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    output = p.stdout.strip()
    logger.debug(
        f"stdout of target determinator for {parent_object}: %s",
        output,
    )
    return output


def compute_annotations_for_object(
    annotator: GitRepoAnnotator, object: str, os_kind: OsKind
) -> tuple[str, str, CommitInclusionState]:
    logger = _LOGGER.getChild("annotate_object").getChild(object).getChild(os_kind)
    logger.debug("Attempting to annotate")
    start = time.time()
    targets = BAZEL_TARGETS[os_kind]

    # The following two external operations were being run in parallel
    # to speed things up, but it turns out one of them often modifies
    # the working directory, making the other one much slower.  Thus,
    # we run them serially now, and -- in between them -- we clean the
    # repository's working directory.
    annotator.checkout(object)
    bazel_query_output = subprocess.check_output(
        [resolve_binary("bazel"), "query", f"deps({targets})"],
        text=True,
        cwd=annotator.dir,
    )
    target_determinator_output = target_determinator(
        annotator.dir, annotator.parent(object), targets
    )
    lap = time.time() - start
    logger.debug("Annotation finished in %.2f seconds", lap)
    return (
        "\n".join(
            [
                line
                for line in bazel_query_output.splitlines()
                if line.strip() and not line.startswith("@")
            ]
        ),
        "\n".join(
            [
                line
                for line in target_determinator_output.splitlines()
                if line.strip() and not line.startswith("@")
            ]
        ),
        (COMMIT_BELONGS if target_determinator_output else COMMIT_DOES_NOT_BELONG),
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
            COMMIT_COULD_NOT_BE_ANNOTATED,
        ], "Expected a specific CommitInclusionState, not %r" % changed
        return typing.cast(CommitInclusionState, changed)


class RecreatingCommitChangeDeterminator(object):
    """
    Computes annotations on the fly from a local Git repository, ignoring existing annotations.

    Annotations made this way are never saved or published to the Git repository.
    """

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

    def commit_changes_artifact(
        self, commit: str, os_kind: OsKind
    ) -> CommitInclusionState:
        """
        Check if the os_kind (artifact) changed in the specifed commit
        by querying the local repo for git notes populated and pushed
        by commit annotator.
        """

        _, _, belongs = compute_annotations_for_object(self.annotator, commit, os_kind)
        assert belongs in [
            COMMIT_BELONGS,
            COMMIT_DOES_NOT_BELONG,
            COMMIT_COULD_NOT_BE_ANNOTATED,
        ], "Expected a specific CommitInclusionState, not %r" % belongs
        return belongs


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
            COMMIT_COULD_NOT_BE_ANNOTATED,
        ], "Expected a specific CommitInclusionState, not %r" % changed
        return typing.cast(CommitInclusionState, changed)
