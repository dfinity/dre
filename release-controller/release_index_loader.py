import os
import pathlib
import subprocess
import sys
import tempfile

from pydantic_yaml import parse_yaml_raw_as
import release_index
from git_repo import GitRepo
from publish_notes import REPLICA_RELEASES_DIR

RELEASE_INDEX_FILE = "release-index.yaml"


def _verify_release_instructions(version):
    return f"""
# IC-OS Verification

To build and verify the IC-OS disk image, run:

```
# From https://github.com/dfinity/ic#verifying-releases
sudo apt-get install -y curl && curl --proto '=https' --tlsv1.2 -sSLO https://raw.githubusercontent.com/dfinity/ic/{version}/gitlab-ci/tools/repro-check.sh && chmod +x repro-check.sh && ./repro-check.sh -c {version}
```

The two SHA256 sums printed above from a) the downloaded CDN image and b) the locally built image, must be identical, and must match the SHA256 from the payload of the NNS proposal.
"""


class ReleaseLoader:
    def __init__(self, release_index_dir: pathlib.Path):
        self.release_index_dir = release_index_dir

    def index(self) -> release_index.Model:
        return parse_yaml_raw_as(release_index.Model, open(self.release_index_dir / RELEASE_INDEX_FILE, "r").read())

    def changelog(self, version: str) -> str | None:
        version_changelog_path = self.release_index_dir / f"{REPLICA_RELEASES_DIR}/{version}.md"
        if version_changelog_path.exists():
            return open(version_changelog_path, "r").read() + _verify_release_instructions(version)

        return None


class DevReleaseLoader(ReleaseLoader):
    def __init__(self):
        dev_repo_root = (
            subprocess.check_output(["git", "rev-parse", "--show-toplevel"], stderr=subprocess.DEVNULL)
            .decode(sys.stdout.encoding)
            .strip()
        )
        if not dev_repo_root:
            raise RuntimeError("Not running in a dev environment")
        super().__init__(pathlib.Path(dev_repo_root))


class GitReleaseLoader(ReleaseLoader):
    def __init__(self, git_repo: str):
        self.git_repo = GitRepo(git_repo)
        super().__init__(self.git_repo.dir)

    def index(self):
        self.git_repo.fetch()
        return super().index()

    def changelog(self, version):
        self.git_repo.fetch()
        return super().changelog(version)


class StaticReleaseLoader(ReleaseLoader):
    def __init__(self, config, changelogs: dict[str, str] = {}):
        self.tempdir = tempfile.TemporaryDirectory()
        super().__init__(pathlib.Path(self.tempdir.name))
        with open(self.release_index_dir / RELEASE_INDEX_FILE, "w") as f:
            f.write(config)
        for v, changelog in changelogs.items():
            with open(self.release_index_dir / f"{v}.md", "w") as f:
                f.write(changelog)

    def __del__(self):
        self.tempdir.cleanup()
