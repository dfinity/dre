import logging

from git_repo import GitRepo
from commit_annotation import (
    GitRepoAnnotator,
    CHANGED_NOTES_NAMESPACES,
    CommitInclusionState,
    COMMIT_BELONGS,
    COMMIT_DOES_NOT_BELONG,
)
from const import OsKind
from commit_annotator import compute_annotations_for_object


_LOGGER = logging.getLogger(__name__)


class RecreatingCommitChangeDeterminator(object):
    """
    Computes annotations on the fly from a local Git repository, ignoring existing annotations.

    Annotations made this way are never saved or published to the Git repository.
    """

    def __init__(self, ic_repo: GitRepo):
        """
        Creates a new commit change determinator.
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

        _, _, belongs = compute_annotations_for_object(self.annotator, commit, os_kind)
        return belongs
