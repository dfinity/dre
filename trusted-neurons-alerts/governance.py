import json
import pathlib
import tempfile

import requests
from ic import Canister
from ic.agent import Agent
from ic.candid import decode
from ic.candid import Types
from ic.certificate import lookup
from ic.client import Client
from ic.identity import Identity
from ic.principal import Principal

DFINITY_NEURON_ID = 27

class GovernanceCanister:
    """A simple client for querying the IC Mainnet Governance canister."""

    def __init__(self):
        """Create a new GovernanceCanister client."""
        self.agent = Agent(Identity(), Client("https://ic0.app"))
        self.principal = "rrkah-fqaaa-aaaaa-aaaaq-cai"
        self.canister = None

    def _version(self):
        """Return the current git version of the Governance canister."""
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
    
    def _get_recent_ballots(self):
        """Return latest 100 proposals voted by Neuron 27."""
        with tempfile.TemporaryDirectory() as tmpdirname:
            version = self._version()
            governance_did = pathlib.Path(tmpdirname) / "governance.did"
            contents = requests.get(
                f"https://raw.githubusercontent.com/dfinity/ic/{version}/rs/nns/governance/canister/governance.did",
                timeout=10,
            ).text
            with open(governance_did, "w", encoding="utf8") as f:
                f.write(contents)

            governance_canister = Canister(agent=self.agent, canister_id=self.principal, candid=open(governance_did, encoding="utf8").read())

            response = governance_canister.get_neuron_info(DFINITY_NEURON_ID)
            recent_ballots = response[0].get('Ok', {}).get('recent_ballots', [])
            return recent_ballots

    def has_dfinity_voted(self, proposal_id: int) -> bool:
        """
        Check if Neuron 27 has recently (latest 100 votes) voted for the proposal with id proposal_id.

        :param proposal_id: The proposal_id to search for.
        :return: True if Neuron 27 have voted on proposal with id proposal_id, otherwise False.
        """
        recent_ballots = self._get_recent_ballots()
        for ballot in recent_ballots:
            for prop in ballot.get('proposal_id', []):
                if prop.get('id') == proposal_id:
                    return True
        
        return False


def main():
    canister = GovernanceCanister()
    print(canister.has_dfinity_voted(0))


if __name__ == "__main__":
    main()
