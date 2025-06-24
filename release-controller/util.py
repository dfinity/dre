# mypy: disable-error-code="unused-ignore"

import argparse
import collections.abc
import hashlib
import logging
import math
import os
import pathlib
import requests
import subprocess
import shutil
import sys
import time
import typing


_LOGGER = logging.getLogger(__name__)


def version_name(rc_name: str, name: str) -> str:
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"


def resolve_binary(name: str) -> str:
    """
    Resolve the binary path for the given binary name.
    Try to locate the binary in expected location if it was packaged in an OCI image.
    """
    me = pathlib.Path(__file__)
    things_to_try = [name + "-embedded", name] if name == "dre" else [name]
    for thing in things_to_try:
        search_path = os.path.pathsep.join(
            # First, look for the binary in the same folder as this file.
            (
                [
                    str(me.parent.parent / "rs" / "cli")
                ]  # Special-casing DRE it's actually in ../rs/cli in prod.
                if name == "dre"
                else [str(me.parent)]
            )
            + [
                # Also search in the regular executable search path.
                os.getenv("PATH", "."),
            ]
        )
        full_path = shutil.which(thing, path=search_path)
        if full_path is not None:
            break
    if full_path is None:
        raise FileNotFoundError(name)
    _LOGGER.debug("Selected %s as binary to run for command %s", full_path, name)
    return full_path


T = typing.TypeVar("T")


# https://stackoverflow.com/a/34482761
def auto_progressbar_with_item_descriptions(
    it: collections.abc.Collection[tuple[str, T]],
    prefix: str = "",
    out: typing.TextIO = sys.stderr,
) -> typing.Generator[T, None, None]:
    """
    Produces a progress bar on standard error if the console is a tty,
    increasing for every item.
    Otherwise it remains silent.

    `it` must be a collection of items (supports __iter__ and __len__).
    Items within `it` must a tuple (description of the item, item)
    The progress bar will display that description when processing that specific item.  Keep the descriptions short.
    """
    count = len(it)
    start = time.time()

    def termsize() -> int:
        try:
            size = os.get_terminal_size()[0]
        except Exception:
            size = 79
        return size

    def show(j: int, desc: str, item: T) -> None:
        size = termsize()

        progress = j / count
        remaining = ((time.time() - start) / j) * (count - j)

        mins, sec = divmod(remaining, 60)
        sec = int(round(sec))
        time_str = f"{int(mins):02}:{sec:02d}"

        pre = f"{prefix}{desc} "
        if not pre.strip():
            pre = ""
        post = f" {j}/{count}, {time_str} to go"
        size = size - len(pre) - len(post) - 2
        progress_width = int(round(progress * size))
        done_width = size - progress_width
        print(
            f"\r{pre}[{'█'*progress_width}{('.'*done_width)}]{post}",
            end="",
            file=out,
            flush=True,
        )

    for i, itemmaybetuple in enumerate(it):
        if isinstance(itemmaybetuple, tuple):
            (desc, item) = itemmaybetuple
        else:
            desc = ""
            item = itemmaybetuple
        yield item
        if sys.stderr.isatty():
            show(i + 1, desc, item)
    if sys.stderr.isatty():
        print(f"\r{' '*(termsize())}", end="\n", flush=True, file=out)


def auto_progressbar(
    it: collections.abc.Collection[T],
    prefix: str = "",
    out: typing.TextIO = sys.stderr,
) -> typing.Generator[T, None, None]:
    """
    Produces a progress bar on standard error if the console is a tty,
    increasing for every item.
    Otherwise it remains silent.

    `it` must be a collection of items (supports __iter__ and __len__).
    """

    class adapt(object):
        def __init__(self, inner: collections.abc.Collection[T]):
            self.inner = inner
            self.len = len(inner)

        def __len__(self) -> int:
            return self.len

        def __iter__(self) -> typing.Iterator[tuple[str, T]]:
            for it in self.inner:
                yield "", it

        def __contains__(self, object: typing.Any) -> bool:
            return self.inner.__contains__(object)

    return auto_progressbar_with_item_descriptions(
        adapt(it),
        prefix,
        out,
    )


class HTTPGenerator(object):
    def __init__(self, response: requests.Response):
        self.resp = response
        self.len = int(response.headers["Content-Length"])
        self.chunk_size = 1024 * 1024

    def __len__(self) -> int:
        return int(math.ceil((self.len / self.chunk_size)))

    def __iter__(self) -> typing.Iterator[bytes]:
        return typing.cast(
            typing.Iterator[bytes], self.resp.iter_content(chunk_size=self.chunk_size)
        )

    def __contains__(self, item: typing.Any) -> bool:
        return False


def sha256sum_http_response(r: requests.Response, prefix: str) -> str:
    h = hashlib.sha256()
    for chunk in auto_progressbar(HTTPGenerator(r), prefix=prefix):
        h.update(chunk)
    return h.hexdigest()


class CustomFormatter(logging.Formatter):
    if sys.stderr.isatty():
        green = "\x1b[32;20m"
        yellow = "\x1b[33;20m"
        blue = "\x1b[34;20m"
        red = "\x1b[31;20m"
        bold_red = "\x1b[31;1m"
        reset = "\x1b[0m"
    else:
        green = ""
        yellow = ""
        blue = ""
        red = ""
        bold_red = ""
        reset = ""
    shortfmt = ":%(name)-20s — %(message)s"
    longfmt = "%(asctime)s %(levelname)13s  %(message)s\n" "%(name)37s"

    FORMATS = {
        logging.DEBUG: blue + "DD" + shortfmt + reset,
        logging.INFO: green + "II" + shortfmt + reset,
        logging.WARNING: yellow + "WW" + shortfmt + reset,
        logging.ERROR: red + "EE" + shortfmt + reset,
        logging.CRITICAL: bold_red + "!!" + shortfmt + reset,
    }

    LONG_FORMATS = {
        logging.DEBUG: blue + longfmt + reset,
        logging.INFO: green + longfmt + reset,
        logging.WARNING: yellow + longfmt + reset,
        logging.ERROR: red + longfmt + reset,
        logging.CRITICAL: bold_red + longfmt + reset,
    }

    def __init__(self, one_line_logs: bool):
        self.one_line_logs = one_line_logs

    def format(self, record: logging.LogRecord) -> str:
        if not self.one_line_logs:
            log_fmt = self.LONG_FORMATS.get(record.levelno)
        else:
            log_fmt = self.FORMATS.get(record.levelno)
        formatter = logging.Formatter(log_fmt)
        return formatter.format(record)


def conventional_logging(one_line_logs: bool, verbose: bool) -> None:
    """
    Set up conventional logging.

    Arguments:
      one_line_logs: make log entries compact and one-line
      verbose: enable debug logging
    """
    root = logging.getLogger()
    root.setLevel(logging.DEBUG if verbose else logging.INFO)
    if verbose:
        for chatty in ["httpcore", "urllib3", "httpx"]:
            logging.getLogger(chatty).setLevel(logging.WARNING)

    ch = logging.StreamHandler()
    ch.setLevel(logging.DEBUG if verbose else logging.INFO)
    ch.setFormatter(CustomFormatter(one_line_logs))
    root.addHandler(ch)


def check_output(cmd: list[str], **kwargs: typing.Any) -> str:
    # _LOGGER.warning("CMD: %s", cmd)
    kwargs = kwargs or {}
    if "text" not in kwargs:
        kwargs["text"] = True
    return typing.cast(str, subprocess.check_output(cmd, **kwargs))


def check_output_binary(cmd: list[str], **kwargs: typing.Any) -> bytes:
    # _LOGGER.warning("CMD: %s", cmd)
    kwargs = kwargs or {}
    kwargs["text"] = False
    return typing.cast(bytes, check_output(cmd, **kwargs))


def check_call(cmd: list[str], **kwargs: typing.Any) -> int:
    # _LOGGER.warning("CMD: %s", cmd)
    return subprocess.check_call(cmd, **kwargs)


def repr_ellipsized(s: str, max_length: int = 80) -> str:
    if len(s) < max_length:
        return repr(s)
    return (
        repr(s[: int(max_length / 2) - 2]) + "..." + repr(s[-int(max_length / 2) - 2 :])
    )


class DefaultSubcommandArgParser(argparse.ArgumentParser):
    __default_subparser: str | None = None

    def set_default_subparser(self, name: str) -> None:
        self.__default_subparser = name

    def _parse_known_args(  # type: ignore
        self,
        arg_strings: list[str],
        namespace: argparse.Namespace,
    ) -> tuple[argparse.Namespace, list[str]]:
        in_args = set(arg_strings)
        d_sp = self.__default_subparser
        if d_sp is not None and not {"-h", "--help"}.intersection(in_args):
            assert self._subparsers is not None
            for x in self._subparsers._actions:
                subparser_found = isinstance(
                    x, argparse._SubParsersAction
                ) and in_args.intersection(x._name_parser_map.keys())
                if subparser_found:
                    break
            else:
                # insert default in first position, this implies no
                # global options without a sub_parsers specified
                arg_strings = [d_sp] + arg_strings
        return super(DefaultSubcommandArgParser, self)._parse_known_args(  # type: ignore
            arg_strings, namespace
        )
