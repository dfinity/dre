import datetime
import logging
import time
import typing
import dre_cli

from const import OsKind, GUESTOS, HOSTOS


LOGGER = logging.getLogger()


class ProposalState(object):
    def __init__(
        self, version_id: str, os_kind: OsKind, state_store: "ReconcilerState"
    ):
        self._os_kind = os_kind
        self._version_id = version_id
        self._store = state_store


class NoProposal(ProposalState):
    def __str__(self) -> str:
        return "No %s proposal for version %s" % (self._os_kind, self._version_id)

    def record_submission(self, proposal_id: int) -> "SubmittedProposal":
        return self._store._record_proposal_id(
            self._version_id, self._os_kind, proposal_id
        )

    def record_malfunction(self) -> "DREMalfunction":
        return self._store._record_malfunction(self._version_id, self._os_kind)


class DREMalfunction(NoProposal):
    def __str__(self) -> str:
        return "Proposal attempt for version %s failed at %s" % (
            self._version_id,
            self._store._get_proposal_age(self._version_id, self._os_kind),
        )

    def ready_to_retry(self) -> bool:
        malfunction_age = self._store._get_proposal_age(self._version_id, self._os_kind)
        remaining_time_until_retry = datetime.timedelta(minutes=10) - (
            datetime.datetime.now() - malfunction_age
        )
        if remaining_time_until_retry.total_seconds() > 0:
            self._store._logger.warning(
                "version %s: earlier proposal submission attempted but most likely failed, will retry in %s seconds",
                self._version_id,
                remaining_time_until_retry.total_seconds(),
            )
            return False
        else:
            return True


class SubmittedProposal(ProposalState):
    def __str__(self) -> str:
        return "Proposal for version %s submitted with ID %s" % (
            self._version_id,
            self.proposal_id,
        )

    def __init__(
        self,
        version_id: str,
        os_kind: OsKind,
        state_store: "ReconcilerState",
        proposal_id: int,
    ):
        super().__init__(version_id, os_kind, state_store)
        self.proposal_id = proposal_id


ProposalRetriever = typing.Callable[
    [str, OsKind], NoProposal | DREMalfunction | SubmittedProposal
]


class ReconcilerState:
    """State for the reconciler. This is used to keep track of the proposals that have been submitted."""

    def __init__(
        self,
        known_proposal_retriever: typing.Callable[
            [],
            tuple[
                dict[str, dre_cli.ElectionProposal],
                dict[str, dre_cli.ElectionProposal],
            ],
        ]
        | None = None,
    ):
        """
        Create a new state object.

        If specified, every proposal mentioned in the known_proposals list will be
        recorded to the state database as existing during initialization.
        """
        self.state: dict[
            str,
            tuple[typing.Literal["submitted"], float, int]
            | tuple[typing.Literal["malfunction"], float],
        ] = {}
        self._logger = logging.getLogger(self.__class__.__name__)
        if known_proposal_retriever:
            self.update_state(known_proposal_retriever)

    def update_state(
        self,
        known_proposal_retriever: typing.Callable[
            [],
            tuple[
                dict[str, dre_cli.ElectionProposal],
                dict[str, dre_cli.ElectionProposal],
            ],
        ],
    ) -> None:
        replica_version_proposals, hostos_version_proposals = known_proposal_retriever()
        for os_kind, version_to_proposal in [
            (typing.cast(OsKind, GUESTOS), replica_version_proposals),
            (typing.cast(OsKind, HOSTOS), hostos_version_proposals),
        ]:
            for version, proposal in version_to_proposal.items():
                p = self.version_proposal(version, os_kind)
                if not isinstance(p, SubmittedProposal):
                    self._logger.debug(
                        "Preemptively recording submission of %s proposal %s for version %s",
                        os_kind,
                        proposal["id"],
                        version,
                    )
                    p.record_submission(proposal["id"])

    def version_proposal(
        self,
        version: str,
        os_kind: OsKind,
    ) -> NoProposal | SubmittedProposal | DREMalfunction:
        """Get the proposal ID for the given version. If the version has not been submitted, return None."""
        res = self.state.get(version + os_kind)
        if res is None:
            return NoProposal(version, os_kind, self)
        elif isinstance(res, tuple) and res[0] == "malfunction":
            return DREMalfunction(version, os_kind, self)
        else:
            return SubmittedProposal(version, os_kind, self, res[2])

    def _get_proposal_age(self, version: str, os_kind: OsKind) -> datetime.datetime:
        state = self.state[version + os_kind]
        return datetime.datetime.fromtimestamp(state[1])

    def _record_malfunction(self, version: str, os_kind: OsKind) -> DREMalfunction:
        """Mark a proposal as submitted."""
        self.state[version + os_kind] = ("malfunction", time.time())
        return DREMalfunction(version, os_kind, self)

    def _record_proposal_id(
        self, version: str, os_kind: OsKind, proposal_id: int
    ) -> SubmittedProposal:
        """Save the proposal ID for the given version."""
        self.state[version + os_kind] = ("submitted", time.time(), proposal_id)
        return SubmittedProposal(version, os_kind, self, proposal_id)
