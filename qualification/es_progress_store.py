#!/usr/bin/env python3
"""Store (and load) progress information in Elasticsearch."""
import datetime
import json
import logging
from datetime import timezone
from typing import Dict
from typing import Optional
from typing import Tuple

import elasticsearch_dsl as es_dsl


ES_NODES = ["https://elasticsearch.testnet.dfinity.network"]

_connected = False


class ProgressStore:
    """Store progress info in ElasticSearch."""

    def __init__(self, index_prefix: str):
        """
        Initialize the ProgressStore.

        It initializes the progress store for the provided index_prefix and ending
        with current year.

        Defer all network work to the very latest moment.
        """
        self._index_prefix = index_prefix if index_prefix.endswith("-") else (index_prefix + "-")
        self.idx_name = f"{self._index_prefix}progress-{datetime.datetime.now().year}"
        # Create the ES index if it does not already exist
        self._since_timestamp = datetime.datetime.now() - datetime.timedelta(days=365)
        self.data: Dict = {}
        self._connected = False

    def _connect(self) -> None:
        global _connected  # pylint: disable=global-statement
        if not _connected:
            # Connection to the cluster needs to be established only once
            es_dsl.connections.create_connection(hosts=ES_NODES, sniff_on_start=False, sniff_on_node_failure=False)
            _connected = True

    def _ensure_index(self) -> None:
        es_dsl_idx = es_dsl.Index(self.idx_name)
        if not es_dsl_idx.exists():
            es_dsl_idx.create()

    def save(
        self,
        doc_id: str,
        timestamp: datetime.datetime,
        data: Optional[Dict] = None,
    ):
        """Save progress."""
        self._connect()
        self._ensure_index()
        if data:
            logging.info("Saving progress: timestamp=%s data=%s", timestamp, data)
        else:
            logging.info("Saving progress: timestamp=%s", timestamp)
        return Progress(
            meta={"id": doc_id},
            timestamp=timestamp,
            data=json.dumps(data or {}),
        ).save(index=self.idx_name)

    def load(self, doc_id: str) -> Tuple[str, Dict]:
        """Load the object from ES."""
        self._connect()
        self._ensure_index()
        entry = Progress.get(id=doc_id, index=self.idx_name, ignore=404)
        if entry and entry.timestamp:
            ts = entry.timestamp
            if ts.tzinfo is None or ts.tzinfo.utcoffset(ts) is None:
                # Elasticsearch itself interprets all datetimes with no
                # timezone information as UTC.
                # https://elasticsearch-dsl.readthedocs.io/en/latest/persistence.html?highlight=timezone#note-on-dates
                ts = ts.replace(tzinfo=timezone.utc)
            self._since_timestamp = ts
            self.data = json.loads(entry.data)
        return (self._since_timestamp.strftime("%Y-%m-%d"), self.data)


class Progress(es_dsl.Document):
    """Progress object stored in ElasticSearch. Used by ProgressStore."""

    id = es_dsl.Text()
    timestamp = es_dsl.Date(default_timezone="UTC")
    data = es_dsl.Text()  # Arbitrary data object


if __name__ == "__main__":
    p = ProgressStore(index_prefix="test-release-qualification")
    print("Loading the last saved timestamp:   ", p.load(doc_id="test-id"))
    ts = datetime.datetime.now() - datetime.timedelta(days=365)
    print("Saving the timestamp: NOW - 365 days", p.load(doc_id="test-id"))
    p.save(doc_id="test-id", timestamp=ts, data={})
    print("Loading the last saved timestamp:   ", p.load(doc_id="test-id"))
