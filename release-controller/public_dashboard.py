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
        the second contains HostOS proposals.  See
        :func:`dre_cli.proposals_by_version` for the aggregation semantics.
        """
        return dre_cli.proposals_by_version(self.get_past_election_proposals())


if __name__ == "__main__":
    cli = DashboardAPI()
    import json

    print(json.dumps(cli.get_past_election_proposals()))
