import json
import logging
import subprocess
import typing
from util import resolve_binary
import os

from const import OsKind, HOSTOS


class Auth(typing.TypedDict):
    key_path: str
    neuron_id: str


LOGGER = logging.getLogger(__name__)


class GuestosElectionProposalPayload(typing.TypedDict):
    replica_version_to_elect: str
    release_package_sha256_hex: str


class HostosElectionProposalPayload(typing.TypedDict):
    hostos_version_to_elect: str
    release_package_sha256_hex: str


class ElectionProposal(typing.TypedDict):
    id: int
    proposer: int
    title: str
    summary: str
    proposal_timestamp_seconds: int
    status: str
    payload: HostosElectionProposalPayload | GuestosElectionProposalPayload


def _mode_flags(dry_run: bool) -> list[str]:
    """
    Return what DRE flag set to use depending on the mode.

    Operations cannot be interactive when used within the callers
    of this module.  Accordingly, depending on the nature of the
    operation, either a flag that specifies *yes, do what I said*
    or *no, just simulate* is necessary.

    This code decides which flags to use.
    """
    return ["--dry-run"] if dry_run else ["--yes"]


class DRECli:
    def __init__(
        self,
        auth: typing.Optional[Auth] = None,
    ):
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

    def _run(self, *args: str, **subprocess_kwargs: typing.Any) -> str:
        """Run the dre CLI."""
        return typing.cast(
            str,
            subprocess.check_output(
                [
                    self.cli,
                    *self.auth,
                    *args,
                ],
                env=self.env,
                text=True,
                **subprocess_kwargs,
            ),
        )

    def get_blessed_guestos_versions(self) -> set[str]:
        """Query the blessed GuestOS versions."""
        return set(
            typing.cast(
                list[str],
                json.loads(
                    subprocess.check_output(
                        [self.cli, "get", "blessed-replica-versions", "--json"],
                        env=self.env,
                    )
                )["value"]["blessed_version_ids"],
            )
        )

    def get_blessed_hostos_versions(self) -> set[str]:
        """Query the blessed HostOS versions."""
        return set(
            typing.cast(
                list[str],
                [
                    n["hostos_version_id"]
                    for n in json.loads(
                        subprocess.check_output([self.cli, "registry"], env=self.env)
                    )["nodes"]
                    if "hostos_version_id" in n and n["hostos_version_id"].strip()
                ],
            )
        )

    def get_past_election_proposals(self) -> list[ElectionProposal]:
        """Get all known GuestOS / HostOS election proposals."""
        return typing.cast(
            list[ElectionProposal],
            json.loads(
                subprocess.check_output(
                    [self.cli, "proposals", "filter", "-t", "ic-os-version-election"],
                    env=self.env,
                )
            ),
        )

    def get_election_proposals_by_version(
        self,
    ) -> tuple[dict[str, ElectionProposal], dict[str, ElectionProposal]]:
        """
        Get all IC OS election proposals in two separate dictionaries keyed
        by version -- the first dictionary contains GuestOS proposals, and
        the second contains HostOS proposals."""
        d: dict[str, ElectionProposal] = {}
        od: dict[str, ElectionProposal] = {}
        known_proposals = self.get_past_election_proposals()
        for proposal in known_proposals:
            for proposal in known_proposals:
                payload = proposal["payload"]
                if "replica_version_to_elect" in payload:
                    replica_version = typing.cast(
                        GuestosElectionProposalPayload, payload
                    ).get("replica_version_to_elect")
                    if not replica_version:
                        continue
                    d[replica_version] = proposal
                if "hostos_version_to_elect" in payload:
                    hostos_version = typing.cast(
                        HostosElectionProposalPayload, payload
                    ).get("hostos_version_to_elect")
                    if not hostos_version:
                        continue
                    od[hostos_version] = proposal
        return d, od

    def propose_to_revise_elected_os_versions(
        self,
        changelog: str,
        version: str,
        os_kind: OsKind,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        dry_run: bool = False,
    ) -> int:
        x = "hostos" if os_kind is HOSTOS else "replica"
        unelect_versions_args = (
            ([f"--{x}-versions-to-unelect"] + list(unelect_versions))
            if len(unelect_versions) > 0
            else []
        )
        self._logger.info("Submitting proposal for version %s", version)
        text = self._run(
            "propose",
            *_mode_flags(dry_run),
            "--proposal-url",
            forum_post_url,
            f"revise-elected-{x}-versions",
            "--proposal-title",
            f"Elect new IC/{os_kind} revision (commit {version[:7]})",
            "--summary",
            changelog,
            "--release-package-sha256-hex",
            package_checksum,
            "--release-package-urls",
            *package_urls,
            f"--{x}-version-to-elect",
            version,
            *unelect_versions_args,
        )
        if not dry_run:
            try:
                return int(text.rstrip().splitlines()[-1].split()[1])
            except ValueError:
                raise ValueError(
                    "The last line of the DRE output did not contain a proposal ID:\n%s"
                    % text
                )
        else:
            # We will not parse the text here.  We dry-ran the thing, after all,
            # so there will be no proposal ID to parse.
            return 0


if __name__ == "__main__":
    cli = DRECli()
    print(cli.get_blessed_hostos_versions())
