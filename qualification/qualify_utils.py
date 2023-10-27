#!/usr/bin/env python3
import datetime
import logging
import os
import pathlib
import resource
import subprocess
import time
import typing

import requests
import tabulate

from factsdb import model
from pylib import ic_utils
from pylib.ic_admin import IcAdmin
from pylib.ic_deployment import IcDeployment

repo_root = ic_utils.repo_root
TABLE_FMT = "pretty"  # https://pypi.org/project/tabulate/


class RunParams:
    """Enclose all parameters needed to run a step."""

    def __init__(self, deployment_name: str, git_revision: str, ic_admin_revision: str):
        """Initialize the object with the deployment name and git revision."""
        self.deployment_name = deployment_name
        self.deployment = IcDeployment(deployment_name)
        ic_admin_mainnet = IcAdmin(git_revision=ic_admin_revision)
        self.mainnet_old_rev_nns = ic_admin_mainnet.get_subnet_replica_version(0)
        if self.mainnet_old_rev_nns == "85ccf68cf19e6dcfd06171e4772eedc2494e6517":
            # The version 85cc has an incorrect sha256sum for the update image
            # So let's use instead another build that doesn't have any functional changes compared to the above
            self.mainnet_old_rev_nns = "47e392e1ebf9199e8db279b61aac250128106d0b"
        self.mainnet_old_rev_app = ic_admin_mainnet.get_subnet_replica_version(6)
        self.git_revision = git_revision
        self.ic_admin = IcAdmin(self.deployment, git_revision=ic_admin_revision)
        model.deployment = self.deployment
        model.db_open_and_load(model.FileStorage())
        self.guests = model.Guest.to_dict()
        self.guests_by_principal = {g["principal"]: g for g in self.guests}
        self.guests_by_ipv6 = {g["ipv6"]: g for g in self.guests}
        self.qualification_stage = ""


def required_dfx_identity_keys_exist():
    principal_key_path_bootstrap = pathlib.Path.home() / ".config/dfx/identity/bootstrap-super-leader/identity.pem"
    if not principal_key_path_bootstrap.exists():
        logging.error("The dfx identity key for staging is missing: %s", principal_key_path_bootstrap)
        return False
    principal_key_path_xnet = pathlib.Path.home() / ".config/dfx/identity/xnet-testing/identity.pem"
    if not principal_key_path_xnet.exists():
        logging.error("The dfx identity key for XNet tests is missing: %s", principal_key_path_xnet)
        return False
    return True


def ensure_revisions_are_blessed(params: RunParams, revisions: typing.List[str]):
    env = os.environ.copy()
    env["TESTNET"] = params.deployment_name
    env["MOTIVATION"] = "Release qualification"
    env["CHANGELOG"] = "* Test change"
    testnet_op = repo_root / "scripts" / "testnet-op"
    cmd_out = subprocess.check_output([testnet_op, "query", "get-blessed-replica-versions"], env=env).decode("utf8")
    for revision in revisions:
        if revision in cmd_out:
            logging.info("Revision %s already blessed", revision)
            continue
        logging.info("Revision %s not in blessed_revs", revision)
        completed = subprocess.run(
            [testnet_op, "propose-to-bless-replica-version", revision],
            env=env,
            input="y\n",
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        for line in completed.stdout.splitlines():
            logging.info("output: %s", line)
        completed.check_returncode()
    return True


def update_replica_revision(params: RunParams, subnet_num: int, revision: str):
    if params.ic_admin.get_subnet_replica_version(subnet_num) == revision:
        return True
    env = os.environ.copy()
    env["TESTNET"] = params.deployment_name
    env["MOTIVATION"] = "Release qualification"
    env["IC_ADMIN"] = params.ic_admin.ic_admin_path
    logging.info("Upgrade subnet %s to revision %s", subnet_num, revision)
    testnet_op = repo_root / "scripts" / "testnet-op"
    completed = subprocess.run(
        [
            testnet_op,
            "propose-to-update-subnet-replica-version",
            str(subnet_num),
            revision,
        ],
        env=env,
        input="y\n",
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    for line in completed.stdout.splitlines():
        logging.info("output: %s", line)
    completed.check_returncode()
    return True


def update_workload_run_test(params: RunParams, load_subnet):
    # Prepare the requirements for running the workload generator
    subnet_members = params.ic_admin.get_subnet(load_subnet)["records"][0]["value"]["membership"]
    subnet_members_ipv6 = [params.guests_by_principal[m]["ipv6"] for m in subnet_members]
    load_urls = [f"http://[{ipv6}]:8080/" for ipv6 in subnet_members_ipv6]
    wg_bin_path = pathlib.Path.home() / "bin" / f"ic-workload-generator.{params.git_revision}"
    # Bump up the ulimit for the number of open files
    resource.setrlimit(resource.RLIMIT_NOFILE, (8192, 8192))
    # Record the start time to query prometheus after the run
    start_timestamp = int(datetime.datetime.now().timestamp())
    runtime_secs = 120
    # Make workload generator timeouts commensurate with the timeout
    # set here.  The default values in engine.rs are 5 minutes for
    # query timeout, and 6 minutes for ingress timeout.
    # Otherwise the program is killed by `subprocess.check_call`
    # before it has had time to finish processing.
    # Discovered when one of the nodes in the staging network
    # was down, which led to a very confusing traceback showing
    # that subprocess.check_call was killing the workload generator.
    timeouts = 59
    subprocess.check_call(
        [
            wg_bin_path,
            ",".join(load_urls),
            "-m=UpdateCounter",
            "-r=100",
            "--payload-size=1k",
            "-n=%s" % runtime_secs,
            "--periodic-output",
            "--query-timeout-secs=%d" % timeouts,
            "--ingress-timeout-secs=%d" % timeouts,
        ],
        timeout=runtime_secs + 60,
    )
    # Record the start time to query prometheus based on timestamps
    end_timestamp = int(datetime.datetime.now().timestamp())
    return update_workload_collect_metrics(
        deployment_name=params.deployment_name,
        subnet_num=load_subnet,
        subnet_members_ipv6=subnet_members_ipv6,
        start_timestamp=start_timestamp,
        end_timestamp=end_timestamp,
    )


def update_workload_collect_metrics(deployment_name, subnet_num, subnet_members_ipv6, start_timestamp, end_timestamp):
    test_duration_secs = end_timestamp - start_timestamp

    metrics_hosts = "|".join(["\\\\[%s\\\\]:9090" % ipv6 for ipv6 in subnet_members_ipv6])
    common_labels = f'ic="{deployment_name}",job="replica",instance=~"{metrics_hosts}"'
    query_selector = (
        'artifact_pool_consensus_height_stat{%s,type="finalization",pool_type="validated",stat="max"}' % common_labels
    )
    # Calculate the averages over the large interval.
    # We split into smaller buckets, then apply avg_over_time. The outer avg it
    # to get an aggregate, instead of having values per replica.
    headers = {"Accept": "application/json"}
    params = {"time": end_timestamp, "query": f"avg(rate({query_selector}[{test_duration_secs}s]))"}
    resp = requests.get(
        "https://ic-metrics-victoria.ch1-obsstage1.dfinity.network/select/0/prometheus/api/v1/query",
        headers=headers,
        params=params,
        timeout=10,
    )
    resp.raise_for_status()
    metrics = resp.json()
    finalization_rate = float(metrics["data"]["result"][0]["value"][1])
    finalization_rate_expected = expected_finalization_rate_for_subnet(subnet_num, len(subnet_members_ipv6)) * 2 / 3
    logging.info(
        "Expected finalization rate %0.2f vs achieved finalization rate %0.2f",
        finalization_rate_expected,
        finalization_rate,
    )

    logging.info("Check the Grafana dashboard (adjust the subnets if necessary)")
    logging.info(
        "Grafana URL: https://grafana.mainnet.dfinity.network/d/ic-progress-clock/ic-progress-clock?orgId=1&var-ic=%s&refresh=30s",
        deployment_name,
    )
    logging.info(
        "Grafana URL: https://grafana.mainnet.dfinity.network/d/execution-metrics/execution-metrics?orgId=1&var-ic=%s",
        deployment_name,
    )

    return finalization_rate >= finalization_rate_expected


def xnet_run_test(params: RunParams):
    e2e_bin_path = pathlib.Path.home() / "bin" / f"e2e-test-driver.{params.git_revision}"

    # Test configuration
    runtime_secs = 60
    request_rate = 10  # Number of requests sent per subnet per round
    num_subnets = 2  # Number of subnets to use
    payload_size = 1024  # Payload size of the individual messages in bytes
    cycles_per_subnet = 10000000000000  # Number of cycles that are available per subnet (for deployments with a wallet)

    env = os.environ.copy()
    env["XNET_TEST_CANISTER_WASM_PATH"] = ic_utils.compute_local_canister_path(
        canister_name="xnet-test-canister", git_revision=params.git_revision
    ).as_posix()

    # Path to the secret key corresponding to the `testing` principal, if required in the deployment
    principal_key = pathlib.Path.home() / ".config/dfx/identity/xnet-testing/identity.pem"

    if not principal_key.exists():
        logging.error("Principal key for XNet testing not found at %s", principal_key)
        logging.error(
            "You can download the key from https://dfinity.1password.com/vaults/frdsbzaohhkmtdhcygw2qqmon4/allitems/3lgcdskqnkipvnsoqkomllxa3m"
        )

    os.chmod(principal_key, 0o400)  # Ensure correct permissions

    cmd = [
        e2e_bin_path,
        "--nns_url",
        params.ic_admin.nns_url,
        "--subnets",
        num_subnets,
        "--principal_key",
        principal_key,
        "--runtime",
        runtime_secs,
        "--rate",
        request_rate,
        "--payload_size",
        payload_size,
        "--cycles_per_subnet",
        cycles_per_subnet,
        "--",
        "4.3",  # The XNet test number in the e2e test driver
    ]
    cmd = [str(x) for x in cmd]
    subprocess.check_call(cmd, timeout=runtime_secs + 60, env=env)

    logging.info("Check the Grafana dashboard (adjust the subnets if necessary)")
    logging.info(
        "Grafana URL: https://grafana.mainnet.dfinity.network/d/xnet/xnet?orgId=1&refresh=5s&from=now-30m&to=now&var-ic=%s",
        params.deployment_name,
    )

    logging.info("Double-check the canisters were created and deleted on both subnets, as expected:")
    logging.info(
        "Grafana URL: https://grafana.mainnet.dfinity.network/d/execution-metrics/execution-metrics?viewPanel=100&orgId=1&from=now-30m&to=now&var-ic=%s",
        params.deployment_name,
    )
    return True


def wait_for_subnet_revision(params: RunParams, subnet_num: int, revision: str):
    try:
        subnet_members = params.ic_admin.get_subnet(subnet_num)["records"][0]["value"]["membership"]
        subnet_members_ipv6 = [params.guests_by_principal[m]["ipv6"] for m in subnet_members]
        for _i in range(100):
            try:
                on_new_revision = []
                still_not_active = []
                for ipv6 in subnet_members_ipv6:
                    node_metrics = requests.get(f"http://[{ipv6}]:9090/metrics", timeout=10).text
                    if revision in node_metrics:
                        on_new_revision.append(params.guests_by_ipv6[ipv6]["name"])
                    else:
                        still_not_active.append(params.guests_by_ipv6[ipv6]["name"])
                if len(on_new_revision) == len(subnet_members):
                    logging.info("Revision %s active on all subnet %s nodes", revision, subnet_num)
                    return True
                else:
                    logging.info(
                        "Revision %s still not active on subnet %s nodes: %s", revision, subnet_num, still_not_active
                    )
            except requests.RequestException as exc:
                logging.error("Failed to make a request: %s", exc)
            time.sleep(10)
    except Exception:
        logging.exception("Failed to get subnet revision")
        raise
    return False


def table_with_subnet_revisions(params: RunParams):
    headers = ["Subnet ID", "Git revision"]
    subnets_versions = params.ic_admin.get_subnet_replica_versions()
    return tabulate.tabulate(subnets_versions.items(), headers, tablefmt=TABLE_FMT, showindex="always")


def expected_finalization_rate_for_subnet(subnet_num, subnet_size):
    """Return the expected finalization rate for the given subnet number (int) and size."""
    XL_subnet_size = 55
    if subnet_size > XL_subnet_size:
        return 0.24
    elif subnet_num == 0:
        return 0.3
    return 0.9
