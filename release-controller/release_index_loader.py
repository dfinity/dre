import pathlib
import re
import subprocess
import sys
import tempfile

import release_index
from git_repo import GitRepo
from publish_notes import REPLICA_RELEASES_DIR
from pydantic_yaml import parse_yaml_raw_as

RELEASE_INDEX_FILE = "release-index.yaml"


def _verify_release_instructions(version):
    return f"""
# IC-OS Verification

To build and verify the IC-OS disk image, run:

```
# From https://github.com/dfinity/ic#verifying-releases
sudo apt-get install -y curl && curl --proto '=https' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/{version}/ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c {version}
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image, must be identical, and must match the SHA256 from the payload of the NNS proposal.
"""


class ReleaseLoader:
    """Load release information from the release index and changelog files."""

    def __init__(self, release_index_dir: pathlib.Path):
        """Create a new ReleaseLoader."""
        self.release_index_dir = release_index_dir

    def index(self) -> release_index.Model:
        """Load the release index from the RELEASE_INDEX_FILE."""
        return parse_yaml_raw_as(
            release_index.Model,
            open(self.release_index_dir / RELEASE_INDEX_FILE, "r").read(),
        )

    def changelog_commit(self, _) -> str:
        """Return the commit hash for the changelog file."""
        return ""

    def changelog_path(self, version: str) -> str:
        """Return the path to the changelog file for the given version."""
        return f"{REPLICA_RELEASES_DIR}/{version}.md"

    def changelog(self, version: str) -> str | None:
        """Return the changelog for the given version."""
        version_changelog_path = self.release_index_dir / self.changelog_path(version)
        if version_changelog_path.exists():
            return open(version_changelog_path, "r").read()

    def proposal_summary(self, version: str) -> str | None:
        """Return the proposal summary for the given version."""
        changelog = self.changelog(version)
        if not changelog:
            return None

        return re.sub(
            r"\n## Excluded Changes.*$",
            f"Full list of changes (including the ones that are not relevant to GuestOS) can be found on [GitHub](https://github.com/dfinity/dre/blob/{self.changelog_commit(version)}/replica-releases/{version}.md).\n",
            changelog,
            flags=re.S,
        ) + _verify_release_instructions(version)


class DevReleaseLoader(ReleaseLoader):
    """Load release information from the current git repository."""

    def __init__(self):
        """Create a new DevReleaseLoader."""
        dev_repo_root = (
            subprocess.check_output(["git", "rev-parse", "--show-toplevel"], stderr=subprocess.DEVNULL)
            .decode(sys.stdout.encoding)
            .strip()
        )
        if not dev_repo_root:
            raise RuntimeError("Not running in a dev environment")
        super().__init__(pathlib.Path(dev_repo_root))


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

    def index(self):
        """Fetch the latest changes from the git repo and load the release index."""
        self.git_repo.fetch()
        return super().index()

    def proposal_summary(self, version):
        """Fetch the latest changes from the git repo and load the proposal summary."""
        self.git_repo.fetch()
        return super().proposal_summary(version)

    def changelog_commit(self, version) -> str:
        """Return the commit hash for the changelog file."""
        return self.git_repo.latest_commit_for_file(self.changelog_path(version))


class StaticReleaseLoader(ReleaseLoader):
    """Load release information from static files."""

    def __init__(self, config, changelogs: dict[str, str] = {}):
        """Create a new StaticReleaseLoader."""
        self.tempdir = tempfile.TemporaryDirectory()
        super().__init__(pathlib.Path(self.tempdir.name))
        with open(self.release_index_dir / RELEASE_INDEX_FILE, "w") as f:
            f.write(config)
        for v, changelog in changelogs.items():
            with open(self.release_index_dir / f"{v}.md", "w") as f:
                f.write(changelog)

    def __del__(self):
        """Clean up the temporary directory."""
        self.tempdir.cleanup()
