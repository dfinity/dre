import os
import logging
from typing import Any, cast

from pydiscourse import DiscourseClient

import threading
import time


LOGGER = logging.getLogger(__name__)


def _env_ttl() -> int:
    try:
        return int(os.environ.get("DISCOURSE_TOPIC_GET_TTL_SECS", "3600"))
    except ValueError:
        return 0


class CachedDiscourse:
    """A thin cached wrapper around DiscourseClient GETs.

    Cached methods:
      - topic_page(topic_id, page)

    Invalidation:
      - invalidate_topic(topic_id) drops cached topic pages for that topic
    """

    def __init__(self, client: DiscourseClient, ttl_seconds: int | None = None) -> None:
        self.client = client
        self._ttl = _env_ttl() if ttl_seconds is None else max(0, int(ttl_seconds))
        self._lock = threading.Lock()
        # key: (topic_id, page) -> (timestamp, payload)
        self._topic_pages: dict[tuple[int, int], tuple[float, dict[str, Any]]] = {}

    # Cached GETs
    # All other methods are delegated to the underlying client

    def topic_page(self, topic_id: int, page: int) -> dict[str, Any]:
        if self._ttl <= 0:
            LOGGER.info("[DISCOURSE_API] GET /t/%s.json?page=%s", topic_id, page + 1)
            raw = cast(
                dict[str, Any],
                self.client._get(f"/t/{topic_id}.json", page=page + 1),  # type: ignore[no-untyped-call]
            )
            # Normalize returned page to requested zero-based page if present
            ret = dict(raw)
            if "page" in ret:
                ret["page"] = page
            return ret
        key = (topic_id, page)
        now = time.time()
        with self._lock:
            item = self._topic_pages.get(key)
            if item:
                ts, value = item
                if now - ts < self._ttl:
                    return value
        # miss or expired
        LOGGER.info("[DISCOURSE_API] GET /t/%s.json?page=%s", topic_id, page + 1)
        raw = cast(
            dict[str, Any],
            self.client._get(f"/t/{topic_id}.json", page=page + 1),  # type: ignore[no-untyped-call]
        )
        # Normalize returned page to requested zero-based page if present
        normalized: dict[str, Any] = dict(raw)
        if "page" in normalized:
            normalized["page"] = page
        with self._lock:
            self._topic_pages[key] = (now, normalized)
            # opportunistic purge of expired entries
            if len(self._topic_pages) % 100 == 0:
                self._purge_expired_unlocked(now)
        return normalized

    def invalidate_topic(self, topic_id: int) -> None:
        with self._lock:
            for k in [k for k in self._topic_pages.keys() if k[0] == topic_id]:
                self._topic_pages.pop(k, None)

    # Pass-through properties used in logs/URLs
    @property
    def host(self) -> str:
        return cast(str, getattr(self.client, "host", ""))

    @property
    def api_username(self) -> str:
        return cast(str, getattr(self.client, "api_username", ""))

    # Fallback: delegate anything else to the underlying client
    def __getattr__(self, name: str) -> Any:
        attr = getattr(self.client, name)
        if callable(attr):

            def _wrapped(*args: Any, **kwargs: Any) -> Any:
                lname = name.lower()
                verb = "CALL"
                if lname.startswith("create"):
                    verb = "POST"
                elif lname.startswith("update"):
                    verb = "PUT"
                elif lname.startswith("delete"):
                    verb = "DELETE"
                elif lname.startswith("get") or lname in {
                    "topics_by",
                    "categories",
                    "category",
                    "_get",
                }:
                    verb = "GET"
                LOGGER.info("[DISCOURSE_API] %s %s", verb, name)
                return attr(*args, **kwargs)

            return _wrapped
        return attr

    def __dir__(self) -> list[str]:
        # expose wrapper attributes + underlying client attributes for better DX
        return sorted(set(list(self.__dict__.keys()) + dir(self.client)))

    def _purge_expired_unlocked(self, now: float) -> None:
        if self._ttl <= 0:
            return
        for k in [
            k for (k, (ts, _)) in self._topic_pages.items() if now - ts >= self._ttl
        ]:
            self._topic_pages.pop(k, None)
