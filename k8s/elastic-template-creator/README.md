# Template creator for ElasticSearch

Template creator for ElasticSearch which should create an index template for logs when elastic service starts

### Example run
```bash
poetry run python template-creator.py <elastic-url> [--index-pattern <pattern>] [--template-name <template-name>] [--shard-size-per-index <num-of-shards>]
```