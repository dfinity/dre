import json
import pathlib
import tempfile
from ic import Canister
from ic.agent import Agent
from ic.certificate import lookup
from ic.client import Client
from ic.identity import Identity
from ic.principal import Principal
import urllib.request
from ic.candid import encode, decode, Types


class GovernanceCanister:
    def __init__(self):
        self.agent = Agent(Identity(), Client("https://ic0.app"))
        self.principal = "rrkah-fqaaa-aaaaa-aaaaq-cai"

    def version(self):
        paths = [
            "canister".encode(),
            Principal.from_str(self.principal).bytes,
            "metadata".encode(),
            "git_commit_id".encode(),
        ]
        tree = self.agent.read_state_raw(self.principal, [paths])
        response = lookup(paths, tree)
        version = response.decode("utf-8").rstrip("\n")
        return version

    def replica_version_proposals(self) -> dict[str, int]:
        with tempfile.TemporaryDirectory() as tmpdirname:
            version = self.version()
            governance_did = pathlib.Path(tmpdirname) / "governance.did"
            urllib.request.urlretrieve(
                f"https://raw.githubusercontent.com/dfinity/ic/{version}/rs/nns/governance/canister/governance.did",
                governance_did,
            )
            self.canister = Canister(agent=self.agent, canister_id=self.principal, candid=open(governance_did).read())
            proposals = self.canister.list_proposals(
                {
                    "include_reward_status": [],
                    "before_proposal": [],
                    "limit": 1000,
                    "exclude_topic": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18],
                    "include_status": [],
                    "omit_large_fields": [True],
                    "include_all_manage_neuron_proposals": [False],
                }
            )
            result = {}
            for p in proposals:
                proposal_info = p["proposal_info"][0]
                proposal = proposal_info.get("proposal", [{}])[0]
                execute_function = proposal.get("action", [{}])[0].get("ExecuteNnsFunction", {})
                if execute_function.get("nns_function", None) != 38:
                    continue

                version = decode(
                    bytes(execute_function["payload"]),
                    # TODO: replace with .did parsing. this record type is present in registry.did
                    retTypes=Types.Record(
                        {
                            "release_package_urls": Types.Vec(Types.Text),
                            "replica_versions_to_unelect": Types.Vec(Types.Text),
                            "replica_version_to_elect": Types.Opt(Types.Text),
                            "guest_launch_measurement_sha256_hex": Types.Opt(Types.Text),
                            "release_package_sha256_hex": Types.Opt(Types.Text),
                        }
                    ),
                )[0]["value"]["replica_version_to_elect"][0]
                if version not in result:
                    result[version] = proposal_info["id"][0]["id"]

            return result


def main():
    canister = GovernanceCanister()
    print(json.dumps(canister.replica_version_proposals(), indent=2))


if __name__ == "__main__":
    main()
