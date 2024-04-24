import argparse
import json
import logging
import sys
from datetime import datetime

import requests


def parse():
    parser = argparse.ArgumentParser(description="Script to take snapshots of a certain index")
    parser.add_argument("elastic_url", help="Elasticsearch url, i.e. http://localhost:9200")
    parser.add_argument("--index-pattern", help="Index patterns to search for", dest="index_pattern", action="append")
    parser.add_argument("repository", help="Repository for storing snapshots")
    parser.add_argument("bucket", help="Bucket used for storing snapshots")
    parser.add_argument("endpoint", help="Endpoint used for storing snapshots")
    parser.add_argument(
        "--base-path",
        help="Optional basepath if the snapshot should be stored in a subfolder in bucket",
        default="",
        dest="base_path",
    )
    parser.add_argument("region", help="Region in which the bucket is located")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger("backup")


def main():
    logger = get_logger()
    args = parse()
    logger.info("Running with following configs: \n%s", args)

    if len(args.index_pattern) == 0:
        logger.error("No indices specified. Specify at least 1")
        sys.exit(1)

    response = requests.get("{elastic}/_snapshot".format(elastic=args.elastic_url))
    if response.status_code != 200:
        logger.error("Couldn't list snapshots due to: %s", response.text)
        sys.exit(1)

    snapshots = dict(response.json())
    logger.info("Found following snapshots: %s", snapshots.keys())

    if args.repository not in snapshots.keys():
        logger.info("Didn't find snapshot repository for '%s'. Creating...", args.repository)
        response = requests.put(
            "{elastic}/_snapshot/{repo}".format(elastic=args.elastic_url, repo=args.repository),
            json={
                "type": "s3",
                "settings": {
                    "bucket": args.bucket,
                    "endpoint": args.endpoint,
                    "region": args.region,
                    "base_path": args.base_path,
                },
            },
        )
        if response.status_code != 200:
            logger.error("Couldn't create snapshot repository due to: %s", response.text)
            sys.exit(1)

        logger.info("Successfully created snapshot repository")
    else:
        logger.info("Snapshot repository already present")

    snapshot_name = datetime.now().strftime("snapshot_%Y%m%d_%H%M%S")
    logger.info("Proceeding with creating snapshot: %s", snapshot_name)
    indices = ",".join(args.index_pattern)
    logger.info("Will include indices: %s", indices)

    response = requests.put(
        "{elastic}/_snapshot/{repo}/{name}?wait_for_completion=true".format(
            elastic=args.elastic_url, repo=args.repository, name=snapshot_name
        ),
        json={"indices": indices},
        headers={"Content-Type": "application/json"},
    )
    if response.status_code != 200:
        logger.error("Couldn't create snapshot due to: \n%s", json.dumps(response.json(), indent=2))
        sys.exit(1)

    logger.info("Successfully created snapshot")


if __name__ == "__main__":
    main()
