import argparse
import json
import logging
import os
import re

from elasticsearch import Elasticsearch

SCRIPT_NAME = "cursor-initializator"
NODE_EXPORTER = "node_exporter"
HOST_NODE_EXPORTER = "host_node_exporter"
JOBS = [NODE_EXPORTER, HOST_NODE_EXPORTER]


def parse():
    parser = argparse.ArgumentParser(description="Script to initialize cursors for vector shards")
    parser.add_argument("elastic_url", help="Elasticsearch url, i.e. http://localhost:9200")
    parser.add_argument("node_filter", help="Node filter for current vector shard i.e. a.*")
    parser.add_argument("output_dir", help="Path to which to initialize cursors")
    parser.add_argument("--index-pattern", help="Index patterns to search for", dest="index_pattern", action="append")
    parser.add_argument("--timeout", help="Timeout for elasticsearch requests", dest="timeout", default=300, type=int)
    parser.add_argument("--suf", help="Optional suffix at the dir names", dest="suf", default="", type=str)
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def main():
    args = parse()
    logger = get_logger()

    logger.info("Initializing elastic client with url: %s", args.elastic_url)
    client = Elasticsearch(args.elastic_url, request_timeout=args.timeout)

    indices = [i for i in list(client.indices.get(index=args.index_pattern)) if not i.startswith(".")]

    total_nodes = {}
    for pattern in args.index_pattern:
        if not any([re.match(pattern, i) for i in indices]):
            logger.info("No indices found for pattern %s", pattern)
            continue

        field = "ic_node"
        if pattern.startswith("boundary"):
            field = "_HOSTNAME"
        nodes = client.search(
            index=pattern, body={"aggs": {"patterns": {"terms": {"field": f"{field}.keyword", "size": 2_000}}}}
        )["aggregations"]["patterns"]["buckets"]

        nodes = [
            i["key"]
            for i in nodes
            if re.match(args.node_filter, i["key"]) and not i["key"].startswith("guest") and not i["key"] == "localhost"
        ]

        if len(nodes) == 0:
            logger.info("No nodes found for pattern %s and filter %s", pattern, args.node_filter)
            continue

        for job in JOBS:
            body = {
                "size": len(nodes),
                "query": {
                    "bool": {
                        "minimum_should_match": 1,
                        "must": [{"term": {"job.keyword": job}}],
                        "should": {"regexp": {f"{field}.keyword": f"{args.node_filter}"}},
                    }
                },
                "collapse": {
                    "field": f"{field}.keyword",
                    "inner_hits": {"name": "most_recent", "size": 1, "sort": [{"timestamp": "desc"}]},
                },
            }

            last_logs = client.search(index=pattern, body=body)["hits"]["hits"]

            nodes_and_cursors = [
                {
                    "node": i["_source"][field],
                    "cursor": i["inner_hits"]["most_recent"]["hits"]["hits"][0]["_id"],
                    "timestamp": i["inner_hits"]["most_recent"]["hits"]["hits"][0]["_source"]["timestamp"],
                }
                for i in last_logs
            ]

            for entry in nodes_and_cursors:
                if entry["node"] not in total_nodes:
                    total_nodes[entry["node"]] = {}
                if job not in total_nodes[entry["node"]]:
                    total_nodes[entry["node"]][job] = entry
                    continue
                if total_nodes[entry["node"]][job]["timestamp"] < entry["timestamp"]:
                    total_nodes[entry["node"]][job] = entry

    created = 0
    logger.info("%s", json.dumps(total_nodes, indent=4, default=str))
    for node in total_nodes:
        for job in total_nodes[node]:
            file_name = total_nodes[node][job]["node"]
            if len(node.split("-")) == 2:
                if job == HOST_NODE_EXPORTER:
                    file_name = f"{node}-host"
                elif job == NODE_EXPORTER:
                    file_name = f"{node}-guest"

            path = os.path.join(args.output_dir, f"{file_name}-{job}{args.suf}")
            if not os.path.exists(path):
                os.mkdir(path)
            else:
                logger.warning("Directory already exists, maybe this shouldn't be overriden? %s", path)

            checkpointer = os.path.join(path, "checkpoint.txt")
            with open(checkpointer, "w", encoding="utf-8") as f:
                f.write(total_nodes[node][job]["cursor"] + "\n")
                created += 1

    logger.info("Successfully initialized cursors %s on path %s", created, args.output_dir)


if __name__ == "__main__":
    main()
