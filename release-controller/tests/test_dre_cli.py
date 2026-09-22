# type: ignore

from dre_cli import DRECli


def _cli(mocker) -> DRECli:
    mocker.patch("dre_cli.resolve_binary", return_value="/bin/true")
    return DRECli()


def test_get_elected_guestos_versions__json_list(mocker) -> None:
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value='["aaa", "bbb"]',
    )
    assert cli.get_elected_guestos_versions() == {"aaa", "bbb"}


def test_get_elected_guestos_versions__old_dict_format(mocker) -> None:
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value='{"value": {"blessed_version_ids": ["aaa", "bbb"]}}',
    )
    assert cli.get_elected_guestos_versions() == {"aaa", "bbb"}


def test_get_elected_guestos_versions__bare_lines(mocker) -> None:
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value="aaa\nbbb\n",
    )
    assert cli.get_elected_guestos_versions() == {"aaa", "bbb"}


def test_get_elected_guestos_versions__pretty_printed_non_json(mocker) -> None:
    # Regression test: ic-admin may emit a pretty-printed array that is not
    # parseable by json.loads (e.g. trailing commas / log noise).  We must not
    # leak brackets, quotes or commas into the version ids -- otherwise the
    # values are passed verbatim to `--versions-to-unelect`, producing
    # malformed arguments like '"b090f12a...",'.
    output = "\n".join(
        [
            "[",
            '  "b090f12a838fc5118002d5e832f459cb4c46c399",',
            '  "77f5bce6c37ee4e149cf7c41fdaef38059a7a058",',
            '  "fb721da900b9e9219773ee312f987971338f7c62",',
            "]",
        ]
    )
    cli = _cli(mocker)
    mocker.patch("subprocess.check_output", return_value=output)
    assert cli.get_elected_guestos_versions() == {
        "b090f12a838fc5118002d5e832f459cb4c46c399",
        "77f5bce6c37ee4e149cf7c41fdaef38059a7a058",
        "fb721da900b9e9219773ee312f987971338f7c62",
    }


# --- get_active_guestos_versions ------------------------------------------
#
# The retire list the reconciler computes must never touch a version the
# registry still references, otherwise the governance canister adopts the
# election proposal and then traps on execution -- failing the proposal
# wholesale, so the version it was meant to elect is not elected either.
# These tests pin the four sources that
# `check_replica_version_invariants` (rs/registry/canister/src/invariants/
# replica_version.rs) reads, so this set cannot drift away from it.

import json  # noqa: E402


def _registry(**overrides) -> str:
    dump = {
        "subnets": [],
        "unassigned_nodes_config": None,
        "standard_engine_replica_version": None,
        "api_bns": [],
    }
    dump.update(overrides)
    return json.dumps(dump)


def test_get_active_guestos_versions__all_four_sources(mocker) -> None:
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value=_registry(
            subnets=[{"replica_version_id": "subnet_version"}],
            unassigned_nodes_config={"replica_version": "unassigned_version"},
            standard_engine_replica_version={
                "new_replica_version_id": "engine_new",
                "old_replica_version_id": "engine_old",
                "deployment_progress": 1.0,
            },
            api_bns=[{"principal": "aaaaa-aa", "version": "api_bn_version"}],
        ),
    )
    assert cli.get_active_guestos_versions() == {
        "subnet_version",
        "unassigned_version",
        "engine_new",
        "engine_old",
        "api_bn_version",
    }


def test_get_active_guestos_versions__keeps_completed_engine_rollout_old_id(
    mocker,
) -> None:
    # Regression test for proposals 144021 / 144042.  At deployment_progress
    # 1.0 every engine already runs the new version, so the old one shows up
    # on no node at all and node telemetry reports it as dead.  The registry
    # keeps referencing it until the *next* rollout rewrites the record, and
    # retiring it before then fails the whole election proposal.
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value=_registry(
            standard_engine_replica_version={
                "new_replica_version_id": "dea4a9afe8bb2a6c324eb661e52204c6054d8e91",
                "old_replica_version_id": "7360f8f35bda2e4754bb7f2258d6852feec268e8",
                "deployment_progress": 1.0,
            },
        ),
    )
    assert "7360f8f35bda2e4754bb7f2258d6852feec268e8" in (
        cli.get_active_guestos_versions()
    )


def test_get_active_guestos_versions__skips_blank_and_missing(mocker) -> None:
    # CloudEngine subnets that follow the standard upgrade train carry a blank
    # replica_version_id -- the standard engine record pins their version
    # instead.  A blank must never reach `--replica-versions-to-unelect`.
    cli = _cli(mocker)
    mocker.patch(
        "subprocess.check_output",
        return_value=_registry(
            subnets=[
                {"replica_version_id": ""},
                {"replica_version_id": "   "},
                {},
                {"replica_version_id": "real_version"},
            ],
            api_bns=[{"principal": "aaaaa-aa"}],
        ),
    )
    assert cli.get_active_guestos_versions() == {"real_version"}


def test_get_active_guestos_versions__tolerates_absent_records(mocker) -> None:
    # A registry dump may omit the optional records entirely (they serialise
    # as null when unset).  That must read as "nothing pinned", not crash.
    cli = _cli(mocker)
    mocker.patch("subprocess.check_output", return_value=json.dumps({}))
    assert cli.get_active_guestos_versions() == set()
