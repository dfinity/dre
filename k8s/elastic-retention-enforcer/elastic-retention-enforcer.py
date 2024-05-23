import argparse
import json
import logging
import re
from datetime import datetime
from datetime import timedelta

from elasticsearch import Elasticsearch
from humanfriendly import parse_size


def parse():
    parser = argparse.ArgumentParser(description="Script to enforce retention policy")
    parser.add_argument(
        "--max-disk-util",
        default="100G",
        help="Maximum disk utilization (supports humanfriendly i.e 16G)",
        dest="max_disk_util",
        type=str,
    )
    parser.add_argument("--max-age", default=30, help="Maximum age of logs to retain in days", dest="max_age", type=int)
    parser.add_argument("elastic_url", help="Elasticsearch url, i.e. http://localhost:9200")
    parser.add_argument("--index-pattern", default="_all", help="Index pattern to search for", dest="index_pattern")
    parser.add_argument("--skip-pattern", help="Index pattern to search for", dest="skip_patterns", action="append")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger("retention")


def apply_age_policy(indices, max_age):
    """
    Apply age policy based on maximum amount of days.

    Returns indices to keep and indices to drop.
    """
    oldest_date_to_keep = datetime.today() - timedelta(days=max_age)
    return [i for i in indices if i["created_at"] < oldest_date_to_keep], [
        i for i in indices if i["created_at"] >= oldest_date_to_keep
    ]


def apply_total_bytes_policy(indices, max_disk_util, logger):
    """
    Apply total bytes policy based on maximum bytes.

    Returns indices to keep and indices to drop.
    """
    popped = list()
    while True:
        total = sum([i["size"] for i in indices])
        logger.info(f"Current total: {total}, current count_to_drop: {len(popped)}, max_disk_util: {max_disk_util}")
        if total > max_disk_util:
            popped.append(indices.pop(0))
        else:
            break

    return popped, indices


def divide_chunks(content, n):

    # looping till length l
    for i in range(0, len(content), n):
        yield content[i : i + n]


def main():
    args = parse()
    args.max_disk_util = parse_size(args.max_disk_util, binary=True)
    logger = get_logger()
    logger.info("Initializing elastic client with url: %s", args.elastic_url)
    args_to_skip = []
    if args.skip_patterns is not None and len(args.skip_patterns) > 0:
        logger.info("Skipping indices with pattern: %s", args.skip_patterns)
        args_to_skip = args.skip_patterns
    client = Elasticsearch(args.elastic_url)

    indices = [i for i in list(client.indices.get(index=args.index_pattern)) if not i.startswith(".")]
    logger.info("Found a total of %s indices with patter %s", len(indices), args.index_pattern)

    indice_stats = client.cat.indices(index=args.index_pattern, h=("i", "ss", "creation.date.string"))
    indices = [re.split(r"\s+", indice) for indice in indice_stats.split("\n")[:-1]]
    parsed_indices = []
    for indice in indices:
        system_index = indice[0].startswith(".")
        should_skip_index = any([re.match(i, indice[0]) for i in args_to_skip])
        has_all_stats = len(indice) == 3
        if system_index:
            logger.info("Skipping system index %s", indice[0])
            continue
        if should_skip_index:
            logger.info("Skipping special index: %s", indice[0])
            continue
        if not has_all_stats:
            logger.info("Received line doesn't contain all data: %s", indice)
            continue
        parsed_indice = {
            "name": indice[0],
            "size": parse_size(indice[1], binary=True),
            "created_at": datetime.strptime(indice[2], "%Y-%m-%dT%H:%M:%S.%fZ"),
        }
        parsed_indices.append(parsed_indice)

    indices = parsed_indices
    logger.info("Total indices found: %s", len(indices))
    indices = sorted(indices, key=lambda x: x["created_at"])

    total_drop, indices = apply_age_policy(indices, args.max_age)
    logger.info("Keeping: %s, Drop: %s", len(indices), len(total_drop))

    to_drop, indices = apply_total_bytes_policy(indices, args.max_disk_util, logger)
    total_drop += to_drop
    logger.info("Keeping: %s, Drop: %s", len(indices), len(total_drop))

    if total_drop:
        logger.info("Dropping following indexes:\n%s", json.dumps(total_drop, indent=2, default=str))
        for chunk in divide_chunks([i["name"] for i in total_drop], 10):
            logger.info("Dropping chunk:\n%s", json.dumps(chunk, indent=2, default=str))
            client.indices.delete(index=[i for i in chunk])
    else:
        logger.info("Nothing to drop")


if __name__ == "__main__":
    main()
