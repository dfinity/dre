import argparse
import json
import logging
import os
import sys

import clickhouse_connect

SCRIPT_NAME = "cursor-initializator"


def parse():
    parser = argparse.ArgumentParser(description="Script to initialize cursors for vector shards")
    parser.add_argument("clickhouse_host", help="Clickhouse host, i.e. localhost")
    parser.add_argument("clickhouse_port", help="Clickhouse port, i.e. 8123")
    parser.add_argument("node_filter", help="Node filter for current vector shard i.e. a.*")
    parser.add_argument(
        "--table",
        help="Table to look in",
        action="append",
        dest="tables",
        default=["ic", "ic_boundary", "certificate_syncer", "certificate_issuer"],
    )
    parser.add_argument("output_dir", help="Path to which to initialize cursors")

    parser.add_argument("--username", help="Clickhouse username, i.e. default", default="default")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def get_distinct_values_query(table, field, pattern):
    return f"""
        SELECT DISTINCT {field}
        FROM {table}
        WHERE {field} LIKE '{pattern}'
    """


def get_last_cursor_for_node(table, ch_filter, field):
    return f"""
    SELECT temp.{field}, temp.utc, __CURSOR, temp.job
    FROM {table}, (
        SELECT {field}, max(toDateTime64(timestamp_utc, 9)) as utc, job
        FROM {table}
        GROUP BY {field}, job
        HAVING {field} LIKE '{ch_filter}'
        ) as temp
    WHERE temp.{field} = {table}.{field} AND temp.utc = {table}.timestamp_utc AND temp.job = {table}.job
"""


def main():
    logger = get_logger()
    args = parse()
    logger.info("Initializing clickhouse client with host: %s and port: %s", args.clickhouse_host, args.clickhouse_port)
    logger.info("Looking for nodes matching %s in tables %s", args.node_filter, str(args.tables))

    client = clickhouse_connect.get_client(host=args.clickhouse_host, port=args.clickhouse_port, username=args.username)

    tables = client.command(
        """
        SELECT name
        FROM system.tables
        """
    )

    logger.info("Found %s tables", len(tables))
    if not all([table in tables for table in args.tables]):
        logger.error("Table %s not found", args.table)
        sys.exit(1)

    logger.info("Table found")

    aggregated = {}

    for table in args.tables:
        logger.info("Looking for nodes in table %s", table)

        field = "_HOSTNAME"
        if table == "ic":
            field = "ic_node"

        command = get_last_cursor_for_node(table, args.node_filter, field)
        logger.info("Executing command: \n%s", command)
        response = client.command(command)

        if not isinstance(response, list):
            # should happen only if the result is empty
            response = []

        mapped = [item for line in response for item in line.split("\n")]

        for i in range(0, len(mapped), 4):
            node = mapped[i]
            timestamp = mapped[i + 1]
            cursor = mapped[i + 2]
            job = mapped[i + 3]

            if node not in aggregated:
                aggregated[node] = {}
            aggregated[node][job] = {
                "cursor": cursor,
                "timestamp": timestamp,
            }

    logger.info("Dumping aggregated cursors: \n%s", json.dumps(aggregated, indent=2, sort_keys=True))
    created = 0
    for node in aggregated:
        for job in aggregated[node]:
            file_name = node
            if len(node.split("-")) == 2:
                if job == "host_node_exporter":
                    file_name = f"{file_name}-host"
                elif job == "node_exporter":
                    file_name = f"{file_name}-guest"

            path = os.path.join(args.output_dir, f"{file_name}-{job}-source")
            if not os.path.exists(path):
                os.mkdir(path)
            else:
                logger.warning("Directory already exists, maybe this shouldn't be overriden? %s", path)

            checkpointer = os.path.join(path, "checkpoint.txt")
            with open(checkpointer, "w", encoding="utf-8") as f:
                f.write(aggregated[node][job]["cursor"] + "\n")
                created += 1

    logger.info("Successfully initialized cursors %s on path %s", created, args.output_dir)


if __name__ == "__main__":
    main()
