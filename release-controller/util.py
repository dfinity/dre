import collections.abc
import hashlib
import math
import os
import pathlib
import sys
import time
import typing
import requests


def version_name(rc_name: str, name: str) -> str:
    date = rc_name.removeprefix("rc--")
    return f"release-{date}-{name}"


def resolve_binary(name: str) -> str:
    """
    Resolve the binary path for the given binary name.
    Try to locate the binary in expected location if it was packaged in an OCI image.
    """
    binary_local = os.path.join(os.path.dirname(__file__), name)
    if os.path.exists(binary_local):
        return binary_local
    binary_local = os.path.join("/rs/cli", name)
    if name == "dre" and os.path.exists(binary_local):
        return binary_local
    return name


def release_controller_cache_directory() -> pathlib.Path:
    return pathlib.Path.home() / ".cache" / "release-controller"


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
            f"{pre}[{'█'*progress_width}{('.'*done_width)}]{post}",
            end="\r",
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
        print(f"\r{' '*(termsize())}", end="\r", flush=True, file=out)


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
