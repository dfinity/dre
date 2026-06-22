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
