#!/usr/bin/env python3
"""
Get logs from a host/guest in a Mercury deployment.

# Usage examples

## Guest logs

```
./get-logs.py --guest fm1-spm06
./get-logs.py --guest fm1-spm06 --follow
```

## Host logs

```
./get-logs.py --host fm1-spm06
./get-logs.py --host fm1-spm06 --follow
```
"""
import argparse
import json
import logging
import sys
import time

import tabulate
from elasticsearch import Elasticsearch
from elasticsearch_follow import ElasticsearchFollow
from elasticsearch_follow import Follower


ELASTIC_SEARCH_URL = "https://elasticsearch.mercury.dfinity.systems"
TABLE_FMT = "simple"
es = Elasticsearch([ELASTIC_SEARCH_URL], sniff_on_start=True, sniff_on_connection_fail=True, sniffer_timeout=60)

ES_QUERY_TEMPLATE = """
{
  "bool": {
    "must": [],
    "filter": [
      {
        "bool": {
          "filter": [
            {
              "query_string": {
                "type": "phrase",
                "query": "{{QUERY}}",
                "lenient": true
              }
            },
            {
              "bool": {
                "must_not": {
                  "bool": {
                    "should": [
                      {
                        "match_phrase": {
                          "message": "journalbeat"
                        }
                      }
                    ],
                    "minimum_should_match": 1
                  }
                }
              }
            },
            {
              "bool": {
                "must_not": {
                  "bool": {
                    "should": [
                      {
                        "match_phrase": {
                          "message": "INFO    [monitoring]"
                        }
                      }
                    ],
                    "minimum_should_match": 1
                  }
                }
              }
            },
            {
              "bool": {
                "must_not": {
                  "bool": {
                    "should": [
                      {
                        "match_phrase": {
                          "message": "Moved proposal Signed"
                        }
                      }
                    ],
                    "minimum_should_match": 1
                  }
                }
              }
            },
            {
              "bool": {
                "must_not": {
                  "bool": {
                    "should": [
                      {
                        "match_phrase": {
                          "message": "Added proposal Signed"
                        }
                      }
                    ],
                    "minimum_should_match": 1
                  }
                }
              }
            }
          ]
        }
      },
      {
        "range": {
          "@timestamp": {
            "gte": "{{SINCE}}",
            "lte": "now"
          }
        }
      }
    ],
    "should": [],
    "must_not": []
  }
}
"""


def es_extract_message(result_json):
    if "hits" in result_json:
        return [
            (x["_source"]["host"]["name"], x["_source"]["@timestamp"], x["_source"]["message"])
            for x in result_json["hits"]["hits"]
        ]
    else:
        logging.error("%s", json.dumps(result_json, indent=2))
        return []


class FollowProcessor:
    """A processor for the streaming search results."""

    def process_line(self, line):
        """Reformat and return a line of the search result."""
        msg = line["message"]
        if (
            "journalbeat" in msg
            or "INFO    [monitoring]" in msg
            or "Moved proposal Signed" in msg
            or "Added proposal Signed" in msg
        ):
            return None
        return line["host"]["hostname"], line["@timestamp"], msg


def main():
    """Do the main work."""

    class HelpfulParser(argparse.ArgumentParser):
        """An argparse parser that prints usage on any error."""

        def error(self, message):
            sys.stderr.write("error: %s\n" % message)
            self.print_help()
            sys.exit(2)

    parser = HelpfulParser()

    parser.add_argument(
        "--host",
        action="store",
        nargs="*",
        help="Search for the host logs",
    )

    parser.add_argument(
        "--guest",
        action="store",
        nargs="*",
        help="Search for the guest logs",
    )

    parser.add_argument(
        "--since",
        action="store",
        default="1h",
        help="Since when to search",
    )

    parser.add_argument(
        "--message",
        action="store",
        default="",
        help="Search for this string in the log message",
    )

    parser.add_argument(
        "--follow",
        action="store_true",
        help="Follow (tail) the output continuously",
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

    if args.since.startswith("2021-"):
        since = args.since
    else:
        since = "now-" + args.since
    hosts_query = ""
    if args.host is not None:
        es_index = "hostos-*"
        if args.host:
            hosts_query = "host.hostname:(" + " OR ".join([f'"{h}"' for h in args.host]) + ")"
    elif args.guest is not None:
        es_index = "guestos-*,filebeat-*,journalbeat-guestos-journal-*"
        if args.guest:
            hosts_query = "host.hostname:(" + " OR ".join([f'"{h}"' for h in args.guest]) + ")"
    query = ES_QUERY_TEMPLATE
    message_query = ""
    if args.message:
        message_query = ""
        if hosts_query:
            message_query += " AND "
        message_query += '(message:"' + args.message + '")'
    query = query.replace("{{QUERY}}", (hosts_query + message_query).replace('"', '\\"'))
    query = query.replace("{{SINCE}}", since)
    query = json.loads(query)

    # elasticsearch functions are not properly parsed/handled by pylint so it reports bogus errors
    # pylint: disable=unexpected-keyword-arg
    if args.follow:
        esLogger = logging.getLogger("elasticsearch")
        esLogger.setLevel(logging.WARNING)

        es_follow = ElasticsearchFollow(elasticsearch=es, query_string=hosts_query + message_query)

        # The Follower is used to get a generator which yields new
        # elements until it runs out. time_delta gives the number of
        # seconds to look into the past.
        processor = FollowProcessor()
        follower = Follower(elasticsearch_follow=es_follow, index=es_index, time_delta=60, processor=processor)

        while True:
            entries = follower.generator()
            for line in entries:
                print(line[0], line[1], line[2])
            time.sleep(3)
    else:
        res = es.search(index=es_index, query=query, request_timeout=30, size=1000)
        table = []
        headers = ["host", "timestamp", "message"]
        for line in es_extract_message(res):
            table.append(line)
        print(tabulate.tabulate(table, headers, tablefmt=TABLE_FMT))


if __name__ == "__main__":
    main()
