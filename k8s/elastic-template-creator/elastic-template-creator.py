import argparse
import json
import logging

import requests


def parse():
    parser = argparse.ArgumentParser(description="Script to create log template on ElasticSearch startup")
    parser.add_argument("elastic_url", help="Elasticsearch url, i.e. http://localhost:9200")
    parser.add_argument(
        "--index-pattern", default="ic-logs-*", help="Index pattern to search for", dest="index_pattern"
    )
    parser.add_argument("--number-of-shards-per-index", help="Number of shards per index", type=int, default=3)
    parser.add_argument("--template-name", help="Template name", default="ic-logs-template")
    return parser.parse_args()


def get_logger():
    FORMAT = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(format=FORMAT, level=logging.INFO)
    return logging.getLogger("retention")


def main():
    logger = get_logger()
    args = parse()

    logger.info("Creating template for index pattern: %s", args.index_pattern)
    logger.info("Shard size per index: %s", args.number_of_shards_per_index)
    logger.info("Template name: %s", args.template_name)
    logger.info("ElasticSearch url: %s", args.elastic_url)

    body = {
        "index_patterns": [args.index_pattern],
        "priority": 1,
        "template": {
            "settings": {
                "number_of_shards": args.number_of_shards_per_index,
            }
        },
    }

    while True:
        try:
            r = requests.put(f"{args.elastic_url}/_index_template/{args.template_name}", json=body)
            if r.status_code == 200 and r.json().get("acknowledged", True):
                logger.info("Template created successfully")
                break
            else:
                logger.error("Failed to create template: %s", r.text)
        except Exception as e:
            logger.error("Failed to create template: %s", e)

    response = requests.get(f"{args.elastic_url}/_index_template/{args.template_name}")
    logger.info("Configured template:\n%s", json.dumps(response.json(), indent=2))


if __name__ == "__main__":
    main()
