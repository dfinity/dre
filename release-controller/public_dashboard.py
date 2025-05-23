import dre_cli
import logging
import requests
import typing


LOGGER = logging.getLogger(__name__)
URL = "https://ic-api.internetcomputer.org/api/v3"


class DashboardAPI:
    def __init__(self) -> None:
        self._logger = LOGGER.getChild(self.__class__.__name__)

    def get_past_election_proposals(self) -> list[dre_cli.ElectionProposal]:
        """Get 50 GuestOS / HostOS election proposals known to the dashboard."""
        r = requests.get(
            URL + "/proposals?limit=50&include_topic=TOPIC_IC_OS_VERSION_ELECTION"
        )
        r.raise_for_status()
        props = r.json()["data"]
        proplist: list[dre_cli.ElectionProposal] = []

        for prop in props:
            prop["proposer"] = int(prop["proposer"])
            prop["id"] = prop["proposal_id"]
            del prop["proposal_id"]
            proplist.append(typing.cast(dre_cli.ElectionProposal, prop))
        return proplist

    def get_election_proposals_by_version(
        self,
    ) -> tuple[
        dict[str, dre_cli.ElectionProposal], dict[str, dre_cli.ElectionProposal]
    ]:
        """
        Get IC OS election proposals in two separate dictionaries keyed
        by version -- the first dictionary contains GuestOS proposals, and
        the second contains HostOS proposals."""

        d: dict[str, dre_cli.ElectionProposal] = {}
        od: dict[str, dre_cli.ElectionProposal] = {}
        known_proposals = self.get_past_election_proposals()
        for proposal in known_proposals:
            for proposal in known_proposals:
                payload = proposal["payload"]
                if "replica_version_to_elect" in payload:
                    replica_version = typing.cast(
                        dre_cli.GuestosElectionProposalPayload, payload
                    ).get("replica_version_to_elect")
                    if not replica_version:
                        continue
                    d[replica_version] = proposal
                if "hostos_version_to_elect" in payload:
                    hostos_version = typing.cast(
                        dre_cli.HostosElectionProposalPayload, payload
                    ).get("hostos_version_to_elect")
                    if not hostos_version:
                        continue
                    od[hostos_version] = proposal
        return d, od


if __name__ == "__main__":
    cli = DashboardAPI()
    import json

    print(json.dumps(cli.get_past_election_proposals()))
