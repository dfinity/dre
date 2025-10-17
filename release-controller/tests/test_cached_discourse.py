from __future__ import annotations

from typing import Any, cast
from _pytest.monkeypatch import MonkeyPatch

from cached_discourse import CachedDiscourse


class StubClient:
    def __init__(self) -> None:
        self.host = "https://example.org"
        self.api_username = "user"
        self.calls: list[tuple[str, tuple[Any, ...], dict[str, Any]]] = []

    def _get(self, path: str, *, page: int) -> dict[str, Any]:
        self.calls.append(("_get", (path,), {"page": page}))
        # Return a page-specific payload so cache hits are observable
        return {"path": path, "page": page, "posts": [page]}


def test_topic_page_caches_and_invalidates(monkeypatch: MonkeyPatch) -> None:
    stub = StubClient()
    cd = CachedDiscourse(cast(Any, stub), ttl_seconds=60)

    # First call hits underlying client
    a = cd.topic_page(123, 0)
    assert a["page"] == 0
    assert stub.calls[-1] == ("_get", ("/t/123.json",), {"page": 1})

    # Second call (same key) served from cache, no new client call
    _ = cd.topic_page(123, 0)
    assert stub.calls[-1] == ("_get", ("/t/123.json",), {"page": 1})

    # Different page should call client
    b = cd.topic_page(123, 1)
    assert b["page"] == 1
    assert stub.calls[-1] == ("_get", ("/t/123.json",), {"page": 2})

    # Invalidate topic; next call should hit client again
    cd.invalidate_topic(123)
    _ = cd.topic_page(123, 0)
    assert stub.calls[-1] == ("_get", ("/t/123.json",), {"page": 1})


def test_topic_page_ttl_expiration(monkeypatch: MonkeyPatch) -> None:
    stub = StubClient()
    cd = CachedDiscourse(cast(Any, stub), ttl_seconds=10)

    # Freeze time by monkeypatching time module inside instance
    import importlib
    from typing import Any as _Any

    mod = cast(_Any, importlib.import_module("cached_discourse"))

    t = {"now": 1000.0}

    def fake_time() -> float:
        return t["now"]

    # Patch time.time used by CachedDiscourse
    orig_time = mod.time.time
    monkeypatch.setattr(mod.time, "time", fake_time)
    try:
        _ = cd.topic_page(321, 0)
        first_call = len(stub.calls)
        # within TTL -> cached
        t["now"] = 1005.0
        _ = cd.topic_page(321, 0)
        assert len(stub.calls) == first_call
        # after TTL -> recalc
        t["now"] = 1011.0
        _ = cd.topic_page(321, 0)
        assert len(stub.calls) == first_call + 1
    finally:
        monkeypatch.setattr(mod.time, "time", orig_time)
