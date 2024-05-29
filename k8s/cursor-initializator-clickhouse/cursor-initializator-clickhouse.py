import argparse
import logging
import os

import clickhouse_connect

SCRIPT_NAME = "cursor-initializator"


def parse():
    parser = argparse.ArgumentParser(description="Script to initialize cursors for vector shards")
    parser.add_argument("url", help="Clickhouse DSN, i.e. https://username@host:port")
    parser.add_argument("--password", help="Clickhouse password, i.e. default", default=os.environ.get("PASSWORD"))
    parser.add_argument("node_filter", help="Node filter for current vector shard i.e. a.*")
    parser.add_argument("output_dir", help="Path to which to initialize cursors")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger(SCRIPT_NAME)


def main():
    logger = get_logger()
    args = parse()
    logger.info("Initializing clickhouse client with URL: %s", args.url)
    logger.info("Looking for nodes matching %s", args.node_filter)

    client = clickhouse_connect.get_client(interface="https", dsn=args.url, password=args.password)

    result = client.query(
        f"""
        SELECT
            `ic_node_id`,
            `cursor`
        FROM
            `ic_boundary_cursor_distributed` FINAL
        WHERE
            match(`ic_node_id`, '{args.node_filter}')
        """
    )

    for r in result.result_rows:
        dir = os.path.join(args.output_dir, f"{r[0]}-node_exporter")

        if not os.path.exists(dir):
            os.mkdir(dir)

        with open(os.path.join(dir, f"checkpoint.txt"), "w", encoding="utf-8") as f:
            f.write(f"{r[1]}\n")

    logger.info("Successfully initialized %d cursors on path %s", len(result.result_rows), args.output_dir)


if __name__ == "__main__":
    main()
