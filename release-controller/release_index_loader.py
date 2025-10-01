import logging
import pathlib
import re
import tempfile

import release_index
from git_repo import GitRepo
from const import OsKind, HOSTOS, GUESTOS
from pydantic_yaml import parse_yaml_raw_as

from publish_notes import release_directory

RELEASE_INDEX_FILE = "release-index.yaml"
LOGGER = logging.getLogger(__name__)


def _verify_release_instructions(
    version: str, os_kind: OsKind, security_fix: bool
) -> str:
    other_os_kind = HOSTOS if os_kind == GUESTOS else GUESTOS
    with_security_caveat = ""
    if security_fix:
        with_security_caveat = "\n_You will be able to follow the instructions below as soon as the source code has been released._\n"

    return f"""
# IC-OS Verification
{with_security_caveat}
To build and verify the IC-OS {os_kind} disk image, after installing curl if necessary (`sudo apt install curl`), run:

```
# From https://github.com/dfinity/ic#verifying-releases
curl -fsSL https://raw.githubusercontent.com/dfinity/ic/{version}/ci/tools/repro-check | python3 - -c {version} --{os_kind.lower()}
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image, must be identical, and must match the SHA256 from the payload of the NNS proposal.

While not required for this NNS proposal, as we are only electing a new {os_kind} version here, you have the option to verify the build reproducibility of the {other_os_kind} by passing `--{other_os_kind.lower()}` to the script above instead of `--{os_kind.lower()}`, or the SetupOS by passing `--setupos`.
"""


class ReleaseLoader:
    """Load release information from the release index and changelog files."""

    def __init__(self, release_index_dir: pathlib.Path):
        """Create a new ReleaseLoader."""
        self.release_index_dir = release_index_dir
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def index(self) -> release_index.Model:
        """Load the release index from the RELEASE_INDEX_FILE."""
        return parse_yaml_raw_as(
            release_index.Model,
            open(self.release_index_dir / RELEASE_INDEX_FILE, "r").read(),
        )

    def changelog_commit(self, version: str, os_kind: OsKind) -> str:
        """Return the commit hash for the changelog file."""
        return ""

    def changelog_path(self, version: str, os_kind: OsKind) -> str:
        """Return the path to the changelog file for the given version."""
        reldir = release_directory(os_kind)
        return f"{reldir}/{version}.md"

    def changelog(self, version: str, os_kind: OsKind) -> str | None:
        """Return the changelog for the given version."""
        version_changelog_path = self.release_index_dir / self.changelog_path(
            version, os_kind
        )
        if version_changelog_path.exists():
            self._logger.debug(
                "Changelog for %s %s is ready for proposal.", os_kind, version
            )
            return open(version_changelog_path, "r").read()
        self._logger.debug(
            "Changelog for %s %s is not ready for proposal; it does not exist yet.",
            os_kind,
            version,
        )
        return None

    def proposal_summary(
        self, version: str, os_kind: OsKind, security_fix: bool
    ) -> str | None:
        """Return the proposal summary for the given version."""
        changelog = self.changelog(version, os_kind)
        if not changelog:
            return None

        reldir = release_directory(os_kind)
        return re.sub(
            r"\n## Excluded Changes.*$",
            f"Full list of changes (including the ones that are not relevant to {os_kind}) can be found on [GitHub](https://github.com/dfinity/dre/blob/{self.changelog_commit(version, os_kind)}/{reldir}/{version}.md).\n",
            changelog,
            flags=re.S,
        ) + _verify_release_instructions(version, os_kind, security_fix)


class DevReleaseLoader(ReleaseLoader):
    """Load release information from the current git repository."""

    def __init__(self, path: str) -> None:
        """
        Create a new DevReleaseLoader.

        Args:
            path (str): The filesystem path to a local git repository.
        """
        super().__init__(pathlib.Path(path))


class GitReleaseLoader(ReleaseLoader):
    """Load release information from a git repository."""

    def __init__(
        self,
        git_repo: str | GitRepo,
    ):
        """Create a new GitReleaseLoader."""
        if isinstance(git_repo, str):
            self.git_repo = GitRepo(git_repo)
        else:
            self.git_repo = git_repo
        super().__init__(self.git_repo.dir)

    def index(self) -> release_index.Model:
        """Fetch the latest changes from the git repo and load the release index."""
        self.git_repo.fetch()
        return super().index()

    def proposal_summary(
        self, version: str, os_kind: OsKind, security_fix: bool
    ) -> str | None:
        """Fetch the latest changes from the git repo and load the proposal summary."""
        self.git_repo.fetch()
        return super().proposal_summary(version, os_kind, security_fix)

    def changelog_commit(self, version: str, os_kind: OsKind) -> str:
        """Return the commit hash for the changelog file."""
        return self.git_repo.latest_commit_for_file(
            self.changelog_path(version, os_kind)
        )


class StaticReleaseLoader(ReleaseLoader):
    """Load release information from static files."""

    def __init__(self, config: str, changelogs: dict[str, str] = {}) -> None:
        """Create a new StaticReleaseLoader."""
        self.tempdir = tempfile.TemporaryDirectory(
            prefix=f"reconciler-{self.__class__.__name__}-", delete=False
        )
        super().__init__(pathlib.Path(self.tempdir.name))
        with open(self.release_index_dir / RELEASE_INDEX_FILE, "w") as f:
            f.write(config)
        for v, changelog in changelogs.items():
            with open(self.release_index_dir / f"{v}.md", "w") as f:
                f.write(changelog)

    def __del__(self) -> None:
        """Clean up the temporary directory."""
        self.tempdir.cleanup()


if __name__ == "__main__":
    ic_repo = GitRepo("https://github.com/DFINITY/ic.git", main_branch="master")
    loader = GitReleaseLoader(ic_repo)
    res = loader.proposal_summary(
        "35bfcadd0f2a474057e42393917b8b3ac269627a",
        security_fix=False,
        os_kind=GUESTOS,
    )
    print(res)
