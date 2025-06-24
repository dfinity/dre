import requests
import typing


class PrometheusSeries(typing.TypedDict):
    metric: dict[str, str]


class PrometheusData(typing.TypedDict):
    result: list[PrometheusSeries]


class PrometheusQueryResponse(typing.TypedDict):
    data: PrometheusData


# This query retrieves a count per replica version of all assigned nodes
# plus a count per replica version of all unassigned nodes
# (both aggregated) so long as the overall ount per replica version is
# greater than 10 nodes, which means we deem said replica version active.
ACTIVE_GUESTOS_VERSIONS_QUERY = """
max_over_time(
  sum by (ic_active_version) (
    label_replace(count by (ic_active_version) (ic_replica_info), "kind", "assigned", "ic_active_version", ".+")
    or
    label_replace(count by (ic_active_version) (ic_orchestrator_info{ic_subnet=""}), "kind", "unassigned", "ic_active_version", ".+")
  )[1h]
) > 10
"""

ACTIVE_HOSTOS_VERSIONS_QUERY = """
max_over_time(
  sum by (version) (
    count by (version) (hostos_version)
  )[1h]
) > 10
"""


class ICPrometheus:
    """A simple client for querying the Internet Computer's Prometheus instance."""

    def __init__(self, url: str) -> None:
        """Create a new ICPrometheus client."""
        self.prometheus_url = url

    def active_guestos_versions(self) -> list[str]:
        """Return a list of active GuestOS versions."""
        versions = [
            r["metric"]["ic_active_version"]
            for r in self.query(ACTIVE_GUESTOS_VERSIONS_QUERY)["data"]["result"]
        ]
        if not versions:
            raise Exception("expected at least one active version")
        return versions

    def active_hostos_versions(self) -> list[str]:
        """Return a list of active HostOS versions."""
        versions = [
            r["metric"]["version"]
            for r in self.query(ACTIVE_HOSTOS_VERSIONS_QUERY)["data"]["result"]
        ]
        if not versions:
            raise Exception("expected at least one active version")
        return versions

    def query(self, query: str) -> PrometheusQueryResponse:
        """Query the Prometheus instance."""
        return typing.cast(
            PrometheusQueryResponse,
            requests.get(
                self.prometheus_url + "/api/v1/query",
                params={"query": query},
                timeout=10,
            ).json(),
        )


def main() -> None:
    icprom = ICPrometheus(
        url="https://victoria.mainnet.dfinity.network/select/0/prometheus"
    )
    print(icprom.active_guestos_versions())
    print(icprom.active_hostos_versions())


if __name__ == "__main__":
    main()
