#!/usr/bin/env python3
import argparse
import os
import pathlib
import sys

mydir = os.path.join(os.path.dirname(__file__))
if mydir not in sys.path:
    sys.path.append(mydir)

from const import (  # noqa: E402
    OS_KINDS,
    GUESTOS,
)
from commit_annotation import (  # noqa: E402
    LocalCommitChangeDeterminator,
    CommitAnnotatorClientCommitChangeDeterminator,
    ChangeDeterminatorProtocol,
)
from commit_annotation_recreator import RecreatingCommitChangeDeterminator  # noqa: E402
from git_repo import GitRepo  # noqa: E402
from release_notes_composer import (  # noqa: E402
    prepare_release_notes,
    OrdinaryReleaseNotesRequest,
    release_notes_html,
)
from util import conventional_logging  # noqa: E402


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate release notes")
    parser.add_argument("base_release_tag", type=str, help="base release tag")
    parser.add_argument("base_release_commit", type=str, help="base release commit")
    parser.add_argument("release_tag", type=str, help="release tag")
    parser.add_argument("release_commit", type=str, help="release commit")
    parser.add_argument(
        "--max-commits",
        type=int,
        default=os.environ.get("MAX_COMMITS", 1000),
        help="Maximum number of commits to include in the release notes.",
    )
    parser.add_argument(
        "--commit-annotator-url",
        type=str,
        dest="commit_annotator_url",
        default="http://localhost:9469/",
        help="Base URL of a commit annotator to use in order to determine commit"
        " relevance for a target when composing release notes; if the value 'local'"
        " is specified, it retrieves annotations using an embedded client that"
        " consults a local Git repository clone of the IC; if 'recreate' is specified"
        " as a value, it ignores any existing annotations and runs a process that"
        " re-annotates every commit involved in the release notes-making process"
        " (this is very slow -- roughly a minute per commit to annotate).",
    )
    parser.add_argument(
        "--verbose",
        "--debug",
        action="store_true",
        dest="verbose",
        help="Bump log level.",
    )
    parser.add_argument(
        "--os-kind",
        default=GUESTOS,
        choices=OS_KINDS,
        help="Release artifact for which the notes should be prepared.",
    )
    parser.add_argument(
        "--output",
        type=pathlib.Path,
        help="Generate an HTML file with the output besides printing"
        " it to standard output, and launch your operating system's HTML viewer;"
        " when running via Bazel, please ensure this is an absolute path.",
    )
    args = parser.parse_args()

    conventional_logging(False, args.verbose)

    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")

    if args.commit_annotator_url is None or args.commit_annotator_url == "local":
        annotator: ChangeDeterminatorProtocol = LocalCommitChangeDeterminator(ic_repo)
    elif args.commit_annotator_url == "recreate":
        annotator = RecreatingCommitChangeDeterminator(ic_repo)
    else:
        annotator = CommitAnnotatorClientCommitChangeDeterminator(
            args.commit_annotator_url
        )

    release_notes = prepare_release_notes(
        OrdinaryReleaseNotesRequest(
            args.release_tag,
            args.release_commit,
            args.base_release_tag,
            args.base_release_commit,
            args.os_kind,
        ),
        os_change_determinator=annotator,
        ic_repo=ic_repo,
        max_commits=args.max_commits,
    )
    print(release_notes)
    if args.output:
        release_notes_html(release_notes, args.output)


if __name__ == "__main__":
    main()
