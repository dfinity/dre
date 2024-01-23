# Retention policy enforcing for Elasticsearch

Currently it supports two policies:
* deleting by age
* deleting by prefered disk utilization

### Age policy
It is always evaluated. Default value is 30 days and it can be overriden with `--max-age` command line parameter

### Disk util policy
It is evaluated only if remaining indexes after the evaluation of age policy have a total greater than desired. It supports [humanfriendly input](https://pypi.org/project/humanfriendly/#a-note-about-size-units). Default value is 100G and it can be overriden with `--max-disk-util` command line parameter

### Example run
```bash
poetry run python retention.py <elastic-url> [--max-age <days>] [--max-disk-util <humanfriendly>]
```