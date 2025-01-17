import datetime
import logging
import pathlib
import os
import typing
import dre_cli


LOGGER = logging.getLogger()


class ProposalState(object):
    def __init__(self, version_id: str, state_store: "ReconcilerState"):
        self._version_id = version_id
        self._store = state_store


class NoProposal(ProposalState):
    def __str__(self) -> str:
        return "No proposal for version %s" % self._version_id

    def record_submission(self, proposal_id: int) -> "SubmittedProposal":
        return self._store._record_proposal_id(self._version_id, proposal_id)

    def record_malfunction(self) -> "DREMalfunction":
        return self._store._record_malfunction(self._version_id)


class DREMalfunction(NoProposal):
    def __str__(self) -> str:
        return "Proposal attempt for version %s failed at %s" % (
            self._version_id,
            self._store._get_proposal_age(self._version_id),
        )

    def ready_to_retry(self) -> bool:
        malfunction_age = self._store._get_proposal_age(self._version_id)
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
        self, version_id: str, state_store: "ReconcilerState", proposal_id: int
    ):
        super().__init__(version_id, state_store)
        self.proposal_id = proposal_id


ProposalRetriever = typing.Callable[
    [str], NoProposal | DREMalfunction | SubmittedProposal
]


class ReconcilerState:
    """State for the reconciler. This is used to keep track of the proposals that have been submitted."""

    def __init__(
        self,
        path: pathlib.Path,
        known_proposal_retriever: typing.Callable[
            [], dict[str, dre_cli.ElectionProposal]
        ]
        | None = None,
    ):
        """
        Create a new state object.

        If specified, every proposal mentioned in the known_proposals list will be
        recorded to the state database as existing during initialization.
        """
        os.makedirs(path, exist_ok=True)
        self.path = path
        self._logger = logging.getLogger(self.__class__.__name__)
        if known_proposal_retriever:
            for replica_version, proposal in known_proposal_retriever().items():
                p = self.version_proposal(replica_version)
                if not isinstance(p, SubmittedProposal):
                    self._logger.debug(
                        "Preemptively recording submission of proposal %s for version %s",
                        proposal["id"],
                        replica_version,
                    )
                    p.record_submission(proposal["id"])

    def _version_path(self, version: str) -> pathlib.Path:
        return self.path / version

    def version_proposal(
        self, version: str
    ) -> NoProposal | SubmittedProposal | DREMalfunction:
        """Get the proposal ID for the given version. If the version has not been submitted, return None."""
        version_file = self._version_path(version)
        if not version_file.exists():
            return NoProposal(version, self)
        with open(version_file, encoding="utf8") as vf:
            try:
                return SubmittedProposal(version, self, int(vf.read()))
            except ValueError:
                return DREMalfunction(version, self)

    def _get_proposal_age(self, version: str) -> datetime.datetime:
        return datetime.datetime.fromtimestamp(
            os.path.getmtime(self._version_path(version))
        )

    def _record_malfunction(self, version: str) -> DREMalfunction:
        """Mark a proposal as submitted."""
        self._version_path(version).touch()
        return DREMalfunction(version, self)

    def _record_proposal_id(self, version: str, proposal_id: int) -> SubmittedProposal:
        """Save the proposal ID for the given version."""
        with open(self._version_path(version), "w", encoding="utf8") as f:
            f.write(str(proposal_id))
        return SubmittedProposal(version, self, proposal_id)
