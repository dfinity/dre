import json
import logging
import subprocess
import typing
from util import resolve_binary
import os


class Auth(typing.TypedDict):
    key_path: str
    neuron_id: str


LOGGER = logging.getLogger(__name__)


class DRECli:
    def __init__(self, auth: typing.Optional[Auth] = None):
        self._logger = LOGGER.getChild(self.__class__.__name__)
        self.env = os.environ.copy()
        if auth:
            self.auth = [
                "--private-key-pem",
                auth["key_path"],
                "--neuron-id",
                auth["neuron_id"],
            ]
        else:
            self.auth = []
        self.cli = resolve_binary("dre")

    def _run(self, *args: str):
        """Run the dre CLI."""
        return subprocess.check_output(
            [self.cli, *(["--yes"] if "propose" in args else []), *self.auth, *args],
            env=self.env,
        )

    def get_blessed_versions(self):
        """Query the blessed versions."""
        return json.loads(
            subprocess.check_output(
                [self.cli, "get", "blessed-replica-versions", "--json"], env=self.env
            )
        )

    def place_proposal(
        self,
        changelog,
        version: str,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        dry_run=False,
    ):
        unelect_versions_args = []
        if len(unelect_versions) > 0:
            unelect_versions_args.append("--replica-versions-to-unelect")
            unelect_versions_args.extend(unelect_versions)
        summary = changelog + f"\n\nLink to the forum post: {forum_post_url}"
        self._logger.info("Submitting proposal for version %s", version)
        self._run(
            "propose",
            "update-elected-replica-versions",
            "--proposal-title",
            f"Elect new IC/Replica revision (commit {version[:7]})",
            "--summary",
            summary,
            *(["--dry-run"] if dry_run else []),  # TODO: replace with system proposer
            "--release-package-sha256-hex",
            package_checksum,
            "--release-package-urls",
            *package_urls,
            "--replica-version-to-elect",
            version,
            *unelect_versions_args,
        )
