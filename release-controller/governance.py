import json
import pathlib
import tempfile
import typing

import requests
from ic import Canister
from ic.agent import Agent
from ic.candid import decode
from ic.candid import Types
from ic.certificate import lookup
from ic.client import Client
from ic.identity import Identity
from ic.principal import Principal


class GovernanceCanister:
    """A simple client for querying the IC Mainnet Governance canister."""

    def __init__(self) -> None:
        """Create a new GovernanceCanister client."""
        self.agent = Agent(Identity(), Client("https://ic0.app"))  # type: ignore[no-untyped-call]
        self.principal = "rrkah-fqaaa-aaaaa-aaaaq-cai"
        self.canister: Canister | None = None

    def version(self) -> str:
        """Return the current git version of the Governance canister."""
        paths = [
            "canister".encode(),
            Principal.from_str(self.principal).bytes,  # type: ignore[no-untyped-call]
            "metadata".encode(),
            "git_commit_id".encode(),
        ]
        tree = self.agent.read_state_raw(self.principal, [paths])  # type: ignore[no-untyped-call]
        response = typing.cast(bytes, lookup(paths, tree))  # type: ignore[no-untyped-call]
        version = response.decode("utf-8").rstrip("\n")
        return version

    def replica_version_proposals(self) -> dict[str, int]:
        """Return a dictionary of replica versions to proposal IDs."""
        with tempfile.TemporaryDirectory() as tmpdirname:
            version = self.version()
            governance_did = pathlib.Path(tmpdirname) / "governance.did"
            contents = requests.get(
                f"https://raw.githubusercontent.com/dfinity/ic/{version}/rs/nns/governance/canister/governance.did",
                timeout=10,
            ).text
            with open(governance_did, "w", encoding="utf8") as f:
                f.write(contents)

            if not self.canister:
                self.canister = Canister(  # type: ignore[no-untyped-call]
                    agent=self.agent,
                    canister_id=self.principal,
                    candid=open(governance_did, encoding="utf8").read(),
                )
            proposals = self.canister.list_proposals(  # type: ignore[attr-defined]
                {
                    "include_reward_status": [],
                    "before_proposal": [],
                    "limit": 1000,
                    "exclude_topic": [
                        1,
                        2,
                        3,
                        4,
                        5,
                        6,
                        7,
                        8,
                        9,
                        10,
                        11,
                        12,
                        14,
                        15,
                        16,
                        17,
                        18,
                    ],
                    "include_status": [],
                    "omit_large_fields": [True],
                    "include_all_manage_neuron_proposals": [False],
                }
            )
            result = {}
            for p in proposals:
                proposal_info = p["proposal_info"][0]
                proposal = proposal_info.get("proposal", [{}])[0]
                execute_function = proposal.get("action", [{}])[0].get(
                    "ExecuteNnsFunction", {}
                )
                if execute_function.get("nns_function", None) != 38:
                    continue

                version = decode(  # type: ignore[no-untyped-call]
                    bytes(execute_function["payload"]),
                    # TODO: replace with .did parsing. this record type is present in registry.did
                    retTypes=Types.Record(  # type: ignore[no-untyped-call]
                        {  # type: ignore[arg-type]
                            "release_package_urls": Types.Vec(Types.Text),  # type: ignore[no-untyped-call, arg-type]
                            "replica_versions_to_unelect": Types.Vec(Types.Text),  # type: ignore[no-untyped-call, arg-type]
                            "replica_version_to_elect": Types.Opt(Types.Text),  # type: ignore[no-untyped-call, arg-type]
                            "guest_launch_measurement_sha256_hex": Types.Opt(  # type: ignore[no-untyped-call]
                                Types.Text  # type: ignore[arg-type]
                            ),
                            "release_package_sha256_hex": Types.Opt(Types.Text),  # type: ignore[no-untyped-call, arg-type]
                        }
                    ),
                )[0]["value"]["replica_version_to_elect"][0]
                if version not in result:
                    result[version] = proposal_info["id"][0]["id"]

            return result


def main() -> None:
    canister = GovernanceCanister()
    print(json.dumps(canister.replica_version_proposals(), indent=2))


if __name__ == "__main__":
    main()
