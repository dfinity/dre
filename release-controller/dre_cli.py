import json
import sys
import logging
import subprocess
import typing
from util import resolve_binary
import os
from pathlib import Path
import tempfile

from const import OsKind, HOSTOS, GUESTOS


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


def proposals_by_version(
    proposals: typing.Iterable[ElectionProposal],
    ignore_ids: typing.Iterable[int] = (),
) -> tuple[dict[str, ElectionProposal], dict[str, ElectionProposal]]:
    """
    Aggregate a flat list of IC OS election proposals into two
    version -> proposal dicts (GuestOS, HostOS), with three semantics that
    cooperate to make ``release-index.yaml``'s ``ignored_proposals`` mechanism
    safe:

    * ``ignore_ids`` is applied **before** aggregation, so dropping a stale
      proposal does not also drop a perfectly good alternative proposal
      targeting the same version.
    * When several proposals target the same version (e.g. a failed attempt
      was resubmitted), the proposal with the largest ``id`` wins -- there
      is no reliance on the iteration order of the input.
    * Proposals with no version in their payload (or with an empty one) are
      ignored.

    Regression test target: the previous implementation built the dict by
    unconditional overwrite inside a buggy nested loop, so whichever
    proposal happened to be assigned last (the oldest given a newest-first
    input) ended up in the dict.  Combined with a stale ``ignored_proposals``
    entry, this caused the reconciler to lose track of a successful
    resubmission and fire a duplicate election proposal on every restart.
    """
    guestos: dict[str, ElectionProposal] = {}
    hostos: dict[str, ElectionProposal] = {}
    ignored: set[int] = set(ignore_ids)

    def keep_if_newer(
        store: dict[str, ElectionProposal],
        version: str,
        proposal: ElectionProposal,
    ) -> None:
        existing = store.get(version)
        if existing is None or proposal["id"] > existing["id"]:
            store[version] = proposal

    for proposal in proposals:
        if proposal["id"] in ignored:
            continue
        payload = proposal["payload"]
        if "replica_version_to_elect" in payload:
            guestos_version = typing.cast(
                GuestosElectionProposalPayload, payload
            ).get("replica_version_to_elect")
            if guestos_version:
                keep_if_newer(guestos, guestos_version, proposal)
        if "hostos_version_to_elect" in payload:
            hostos_version = typing.cast(
                HostosElectionProposalPayload, payload
            ).get("hostos_version_to_elect")
            if hostos_version:
                keep_if_newer(hostos, hostos_version, proposal)
    return guestos, hostos


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

    def get_elected_guestos_versions(self) -> set[str]:
        """Query the elected GuestOS versions."""
        # `dre get` forwards its arguments verbatim to `ic-admin`, so the
        # output format is dictated by the ic-admin build that matches the
        # currently deployed registry canister (downloaded at runtime), not
        # by this code. With the migration away from blessed versions, ic-admin
        # has had a few formats. Support all of them here.
        output = subprocess.check_output(
            [self.cli, "get", "elected-guestos-versions", "--json"],
            env=self.env,
            text=True,
        ).strip()
        try:
            parsed = json.loads(output)
        except json.JSONDecodeError:
            parsed = None

        # Old format
        if isinstance(parsed, dict):
            return set(
                typing.cast(list[str], parsed["value"]["blessed_version_ids"])
            )
        # New format, with --json support
        elif isinstance(parsed, list):
            return set(parsed)
        # New format, without --json support.  The output may be either one
        # bare version per line, or a pretty-printed JSON-ish array that is not
        # quite valid JSON (e.g. it carries log noise or trailing commas that
        # make json.loads fail above).  Normalise each line by stripping any
        # surrounding brackets, quotes and commas so that we never leak those
        # characters into the version ids passed to `--versions-to-unelect`.
        else:
            versions = set()
            for line in output.splitlines():
                token = line.strip().strip("[]").strip().strip(",").strip().strip('"')
                if token:
                    versions.add(token)
            return versions

    def get_elected_hostos_versions(self) -> set[str]:
        """Query the elected HostOS versions."""
        return set(
            typing.cast(
                list[str],
                [
                    n["hostos_version_id"]
                    for n in json.loads(
                        subprocess.check_output([self.cli, "registry"], env=self.env)
                    )["elected_host_os_versions"]
                    if "hostos_version_id" in n and n["hostos_version_id"].strip()
                ],
            )
        )

    def get_active_hostos_versions(self) -> set[str]:
        """Query the HostOS versions of every node record in the registry."""
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
        the second contains HostOS proposals.  See
        :func:`proposals_by_version` for the aggregation semantics.
        """
        return proposals_by_version(self.get_past_election_proposals())

    def propose_to_revise_elected_os_versions(
        self,
        changelog: str,
        version: str,
        os_kind: OsKind,
        forum_post_url: str,
        unelect_versions: list[str],
        package_checksum: str,
        package_urls: list[str],
        launch_measurements: typing.Optional[bytes],
        dry_run: bool = False,
    ) -> int:
        x = "hostos" if os_kind == HOSTOS else "guestos"
        y = "hostos" if os_kind == HOSTOS else "replica"
        unelect_versions_args = (
            ([f"--{y}-versions-to-unelect"] + list(unelect_versions))
            if len(unelect_versions) > 0
            else []
        )
        args = [
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
            f"--{y}-version-to-elect",
            version,
            *unelect_versions_args,
        ]

        temp_measurements = tempfile.NamedTemporaryFile()

        # TODO: generalize when the HOSTOS launch measurements are
        #       supported in the ic-admin.
        if os_kind == GUESTOS:
            if launch_measurements is None:
                raise ValueError("Guest launch measurements missing. Cannot proceed.")

            temp_measurements.write(launch_measurements)
            temp_measurements.flush()

            args.extend(["--guest-launch-measurements-path", temp_measurements.name])

        self._logger.info(
            "Submitting proposal for version %s using args: %s",
            version,
            " ".join(args),
        )
        text = self._run(*args)
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
            print(text, file=sys.stderr)
            return 0


if __name__ == "__main__":
    cli = DRECli()
    print("Elected GuestOS", cli.get_elected_guestos_versions())
    print("Elected HostOS ", cli.get_elected_hostos_versions())
    print("Active HostOS  ", cli.get_active_hostos_versions())
