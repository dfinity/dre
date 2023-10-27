#!/usr/bin/env python3
import argparse
import inspect
import logging
import re
import sys
import time

import __fix_import_paths  # noqa # pylint: disable=unused-import
import qualify_utils as qu
import tabulate

from pylib import ic_utils


def step_1a_ensure_blessed_revisions(params: qu.RunParams):
    """Set the version running on subnet 0."""
    git_revs = [params.mainnet_old_rev_nns, params.mainnet_old_rev_app, params.git_revision]
    return qu.ensure_revisions_are_blessed(params, git_revs)


def step_1b_ensure_required_dfx_keys(params: qu.RunParams):
    # pylint: disable=unused-argument
    """Check if the required dfx keys are present."""
    return qu.required_dfx_identity_keys_exist()


def step_2a_upgrade_to_old_replica_revision(params: qu.RunParams):
    """Upgrade to the old ("before upgrade") replica revisions."""
    qu.update_replica_revision(params, 1, params.mainnet_old_rev_app)
    qu.update_replica_revision(params, 2, params.mainnet_old_rev_app)
    return True


def step_2b_upgrade_to_old_replica_revision_wait(params: qu.RunParams):
    """Wait for the upgrade to the old replica revisions to finish."""
    if not qu.wait_for_subnet_revision(params, 1, params.mainnet_old_rev_app):
        return False
    if not qu.wait_for_subnet_revision(params, 2, params.mainnet_old_rev_app):
        return False
    return True


def step_2c_upgrade_to_old_replica_revision(params: qu.RunParams):
    """Upgrade to the old ("before upgrade") replica revisions."""
    qu.update_replica_revision(params, 0, params.mainnet_old_rev_nns)
    return True


def step_2d_upgrade_to_old_replica_revision_wait(params: qu.RunParams):
    """Wait for the upgrade to the old replica revisions to finish."""
    if not qu.wait_for_subnet_revision(params, 0, params.mainnet_old_rev_nns):
        return False
    return True


def step_3_inform_status(params: qu.RunParams):
    """Inform on the current status of the staging."""
    logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_4a_upgrade_app_subnets_to_new_replica_revision(params: qu.RunParams):
    """Upgrade to the new ("after upgrade") replica revisions."""
    params.qualification_stage = "Staging: Upgrade test"  # This test will be marked as failed on any subsequent error
    qu.update_replica_revision(params, 1, params.git_revision)
    return True


def step_4b_upgrade_app_subnets_to_new_replica_revision_wait(params: qu.RunParams):
    """Wait for the upgrade to finish."""
    params.qualification_stage = "Staging: Upgrade test"
    if not qu.wait_for_subnet_revision(params, 1, params.git_revision):
        return False
    logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_4c_upgrade_nns_to_app_subnet_revision(params: qu.RunParams):
    """If the NNS is not the same version as the app subnet, ensure the NNS can upgrade to the app subnet version."""
    if params.mainnet_old_rev_nns != params.mainnet_old_rev_app:
        # This test will be marked as failed on any subsequent error
        params.qualification_stage = "Staging: Upgrade test"
        qu.update_replica_revision(params, 0, params.mainnet_old_rev_app)
        if not qu.wait_for_subnet_revision(params, 0, params.mainnet_old_rev_app):
            return False
        logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_4d_upgrade_nns_to_new_replica_revision(params: qu.RunParams):
    """Upgrade to the new ("after upgrade") replica revisions."""
    params.qualification_stage = "Staging: Upgrade test"  # This test will be marked as failed on any subsequent error
    qu.update_replica_revision(params, 0, params.git_revision)
    return True


def step_4e_upgrade_nns_to_new_replica_revision_wait(params: qu.RunParams):
    """Wait for the upgrade to finish."""
    params.qualification_stage = "Staging: Upgrade test"
    if not qu.wait_for_subnet_revision(params, 0, params.git_revision):
        return False
    logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_5a_download_test_dependencies(params: qu.RunParams):
    """Download test dependencies."""
    for executable in ["ic-workload-generator", "e2e-test-driver"]:
        ic_utils.download_ic_executable(
            git_revision=params.git_revision,
            executable_name=executable,
            blessed=False,
        )
    for canister in ["xnet-test-canister"]:
        ic_utils.download_ic_canister(
            git_revision=params.git_revision,
            canister_name=canister,
            blessed=False,
        )
    return True


def step_5b_run_update_workload(params: qu.RunParams):
    """Run the update workload on the app subnet."""
    params.qualification_stage = "Staging: Upgrade test"
    return qu.update_workload_run_test(params, load_subnet=1)


def step_5c_run_xnet_test(params: qu.RunParams):
    params.qualification_stage = "Staging: XNet test"
    if qu.xnet_run_test(params):
        return True
    return False


def step_6a_downgrade_to_old_replica_revision(params: qu.RunParams):
    """Downgrade to the old ("before") replica revisions."""
    params.qualification_stage = "Staging: Downgrade test"
    qu.update_replica_revision(params, 1, params.mainnet_old_rev_app)
    return True


def step_6b_downgrade_to_old_replica_revision_wait(params: qu.RunParams):
    """Wait for the downgrade to the old ("before") replica revisions to finish."""
    params.qualification_stage = "Staging: Downgrade test"
    if not qu.wait_for_subnet_revision(params, 1, params.mainnet_old_rev_app):
        return False
    logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_6c_downgrade_to_old_replica_revision(params: qu.RunParams):
    """Downgrade to the old ("before") replica revisions."""
    params.qualification_stage = "Staging: Downgrade test"
    qu.update_replica_revision(params, 0, params.mainnet_old_rev_nns)
    return True


def step_6d_downgrade_to_old_replica_revision_wait(params: qu.RunParams):
    """Wait for the downgrade to the old ("before") replica revisions to finish."""
    params.qualification_stage = "Staging: Downgrade test"
    if not qu.wait_for_subnet_revision(params, 0, params.mainnet_old_rev_nns):
        return False
    logging.info("Subnets run the following revisions:\n%s", qu.table_with_subnet_revisions(params))
    return True


def step_7_run_update_workload(params: qu.RunParams):
    """Run the update workload on the app subnet."""
    for executable in ["ic-workload-generator"]:
        ic_utils.download_ic_executable(
            git_revision=params.git_revision,
            executable_name=executable,
            blessed=True,
        )
    params.qualification_stage = "Staging: Downgrade test"
    return qu.update_workload_run_test(params, load_subnet=1)


TABLE_FMT = "simple"  # https://pypi.org/project/tabulate/


def main():
    class HelpfulParser(argparse.ArgumentParser):
        def error(self, message):
            sys.stderr.write("error: %s\n" % message)
            self.print_help()
            sys.exit(2)

    parser = HelpfulParser()

    parser.add_argument(
        "--deployment-name",
        action="store",
        default="staging",
        help="Deployment name (default: staging)",
    )

    parser.add_argument(
        "--git-revision",
        action="store",
        help="Git revision to qualify.",
    )

    parser.add_argument(
        "--ic-admin-revision",
        action="store",
        default="28b13ef800046c4c7befc0a936411ba70b8f908e",
        help="Version of ic-admin to use",
    )

    parser.add_argument(
        "--list-steps",
        "--ls",
        "--dry-run",
        action="store_true",
        help="A list of steps.",
    )

    parser.add_argument(
        "--step-filter",
        action="store",
        nargs="+",
        default="step",
        help="A regular expression filter for the steps to run.",
    )

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose mode")

    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    if len(sys.argv) <= 1:
        parser.print_help()
        sys.exit(0)

    ic_admin_revision = args.ic_admin_revision or args.git_revision
    params = qu.RunParams(
        deployment_name=args.deployment_name, git_revision=args.git_revision, ic_admin_revision=ic_admin_revision
    )

    # Find all functions that start with "step_" to support selective running from the command line
    all_steps = [
        obj
        for name, obj in inspect.getmembers(sys.modules[__name__])
        if (inspect.isfunction(obj) and name.startswith("step_") and obj.__module__ == __name__)
    ]

    if args.list_steps:
        table = []
        headers = ["Step name", "Description"]
        for step in all_steps:
            if any([re.search(_, step.__name__) for _ in args.step_filter]):
                table.append([step.__name__, step.__doc__ or ""])
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))
        sys.exit(0)

    exit_code = 0
    if args.step_filter:
        for step in all_steps:
            if any([re.search(_, step.__name__) for _ in args.step_filter]):
                logging.info("\n\n%s\nRunning: %s\n%s", "*" * 80, step.__name__, "*" * 80)
                start = time.time()
                try:
                    result = step(params=params)
                except Exception:  # pylint: disable=broad-except
                    logging.exception("Execution of step %s failed", step.__name__)
                    result = False
                end = time.time()
                if result:
                    logging.info("Success %s in %.2f seconds", step.__name__, end - start)
                else:
                    logging.error("Failure %s in %.2f seconds", step.__name__, end - start)
                    exit_code = 1
                    break
    else:
        parser.print_help()
        print("\n")
        logging.info("No step filter provided. List of all steps:")
        table = []
        headers = ["Step name", "Description"]
        for step in all_steps:
            table.append([step.__name__, step.__doc__ or ""])
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))

    return exit_code


if __name__ == "__main__":
    exit_code = main()
    sys.exit(exit_code)
