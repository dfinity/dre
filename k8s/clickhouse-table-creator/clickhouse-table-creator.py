import argparse
import json
import logging

import clickhouse_connect

SCRIPT_NAME = "cursor-initializator"


def parse():
    parser = argparse.ArgumentParser(description="Script to initialize tables for clickhouse")
    parser.add_argument("host", help="Clickhouse host, i.e. localhost")
    parser.add_argument("port", help="Clickhouse port, i.e. 8123")
    parser.add_argument("--username", help="Clickhouse username, i.e. default", default="default")
    parser.add_argument("--table", help="Tables to create", dest="table", action="append")
    parser.add_argument("--retention", help="Retention for tables in days", dest="retention", default=30)
    parser.add_argument(
        "--cluster-name", help="Retention for tables in days", dest="cluster_name", default="replicated"
    )
    parser.add_argument("path", help="Path to json with table path")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def get_command_string_for_table(table_name, table_structure, retention, cluster_name):
    columns = ",\n\t    ".join([f"`{column}` {table_structure[column]}" for column in table_structure])
    command = f"""
        CREATE TABLE default.{table_name} ON CLUSTER {cluster_name}
        (
            `timestamp_utc` DateTime64(9, 'UTC'),
            `timestamp` DateTime,
            {columns}
        )
        ENGINE = ReplicatedMergeTree
        ORDER BY (timestamp)
        TTL timestamp + INTERVAL {retention} DAY
    """
    return command


def main():
    logger = get_logger()
    args = parse()

    logger.info("Initializing clickhouse client with url: %s:%s", args.host, args.port)
    logger.info("Creating tables: %s", str(args.table))
    logger.info("Retention: %s", args.retention)
    logger.info("Table structure path: %s", args.path)

    logger.info("Parsing table structure")
    table_structure = {}
    with open(args.path, "r", encoding="utf-8") as f:
        table_structure = json.load(f)

    logger.info("Connecting to clickhouse")
    client = clickhouse_connect.get_client(host=args.host, port=args.port, username=args.username)

    tables = client.command(
        """
        SELECT name
        FROM system.tables
                            """
    ).split("\n")

    logger.info("Found %s tables", len(tables))
    logger.info("Tables: %s", tables)

    for table in args.table:
        if table in tables:
            logger.info("Table %s already exists", table)
        else:
            logger.info("Creating table %s", table)
            logger.info(get_command_string_for_table(table, table_structure[table], args.retention, args.cluster_name))

            try:
                client.command(
                    get_command_string_for_table(table, table_structure[table], args.retention, args.cluster_name)
                )
            except Exception as e:
                logger.error("Error while creating table %s: %s", table, e)


if __name__ == "__main__":
    main()
