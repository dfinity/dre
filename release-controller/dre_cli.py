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
        self.cli = resolve_binary("old-dre")

    def _run(self, *args: str, **subprocess_kwargs: typing.Any) -> str:
        """Run the dre CLI."""
        return typing.cast(
            str,
            subprocess.check_output(
                [
                    self.cli,
                    *(["--yes"] if "propose" in args else []),
                    *self.auth,
                    *args,
                ],
                env=self.env,
                text=True,
                **subprocess_kwargs,
            ),
        )

    def get_blessed_versions(self) -> list[str]:
        """Query the blessed versions."""
        return typing.cast(
            list[str],
            json.loads(
                subprocess.check_output(
                    [self.cli, "get", "blessed-replica-versions", "--json"],
                    env=self.env,
                )
            )["value"]["blessed_version_ids"],
        )

    def get_blessed_hostos_versions(self) -> set[str]:
        """Query the blessed versions."""
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
        """Get all known GuestOS election proposals."""
        return typing.cast(
            list[ElectionProposal],
            json.loads(
                subprocess.check_output(
                    [self.cli, "proposals", "filter", "-t", "ic-os-version-election"],
                    env=self.env,
                )
            ),
        )

    def get_election_proposals_by_version(self) -> dict[str, ElectionProposal]:
        """Get all GuestOS election proposals keyed by version."""
        d: dict[str, ElectionProposal] = {}
        known_proposals = self.get_past_election_proposals()
        for proposal in known_proposals:
            for proposal in known_proposals:
                payload = proposal["payload"]
                if "replica_version_to_elect" not in payload:
                    continue
                replica_version = typing.cast(
                    GuestosElectionProposalPayload, payload
                ).get("replica_version_to_elect")
                if not replica_version:
                    continue
                if replica_version in d:
                    continue
                d[replica_version] = proposal
        return d

    def propose_to_revise_elected_guestos_versions(
        self,
        changelog: str,
        version: str,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        dry_run: bool = False,
    ) -> int:
        try:
            self._run(
                "propose",
                "revise-elected-guestos-versions",
                "--help",
                stderr=subprocess.STDOUT,
            )
            subcommand_name: (
                typing.Literal["update-elected-replica-versions"]
                | typing.Literal["revise-elected-guestos-versions"]
            ) = "revise-elected-guestos-versions"
            # New style of proposal naming is now active in ic-admin.
        except subprocess.CalledProcessError:
            # Old style of proposal naming is still active in ic-admin.
            try:
                self._run(
                    "propose",
                    "revise-elected-guestos-versions",
                    "--help",
                    stderr=subprocess.STDOUT,
                )
            except subprocess.CalledProcessError:
                raise RuntimeError(
                    "No variant of the dre propose command "
                    "can be used to revise elected GuestOS versions"
                )
            subcommand_name = "update-elected-replica-versions"

        return self._propose_to_update_elected_replica_versions(
            subcommand_name,
            changelog,
            version,
            forum_post_url,
            unelect_versions,
            package_checksum,
            package_urls,
            dry_run,
        )

    def _propose_to_update_elected_replica_versions(
        self,
        subcommand_name: typing.Literal["update-elected-replica-versions"]
        | typing.Literal["revise-elected-guestos-versions"],
        changelog: str,
        version: str,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        dry_run: bool = False,
    ) -> int:
        unelect_versions_args = []
        if subcommand_name == "revise-elected-guestos-versions":
            proposal_url_args: list[str] = ["--proposal-url", forum_post_url]
            summary = changelog
        else:
            proposal_url_args = []
            summary = changelog + f"\n\nLink to the forum post: {forum_post_url}"

        if len(unelect_versions) > 0:
            unelect_versions_args.append("--replica-versions-to-unelect")
            unelect_versions_args.extend(unelect_versions)

        self._logger.info("Submitting proposal for version %s", version)
        text = self._run(
            "propose",
            subcommand_name,
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
            *proposal_url_args,
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
