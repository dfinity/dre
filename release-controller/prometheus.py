import requests
import typing


class PrometheusSeries(typing.TypedDict):
    metric: dict[str, str]


class PrometheusData(typing.TypedDict):
    result: list[PrometheusSeries]


class PrometheusQueryResponse(typing.TypedDict):
    data: PrometheusData


class ICPrometheus:
    """A simple client for querying the Internet Computer's Prometheus instance."""

    def __init__(self, url: str) -> None:
        """Create a new ICPrometheus client."""
        self.prometheus_url = url

    def active_versions(self) -> list[str]:
        """Return a list of active versions."""
        versions = [
            r["metric"]["ic_active_version"]
            for r in self.query(
                'max_over_time((count by (ic_active_version) (ic_replica_info or topk(1, ic_orchestrator_info{ic_subnet=""})))[1h])'
            )["data"]["result"]
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
    print(icprom.active_versions())


if __name__ == "__main__":
    main()
