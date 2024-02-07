# Multiservice discovery

Service discovery is built and maintained by
[@dre-team](https://dfinity.enterprise.slack.com/archives/C05LD0CEAHY).

It was built to support multiple networks. By default it will watch only
`mercury` (the IC mainnet), but you can use the HTTP API to add custom
networks on the fly.

## CI and builds

Containers built by the DRE repository CI are published to [GHCR](https://ghcr.io/dfinity/dre/).
They are only built from PRs stemming from branches named `container-*`.

## Test

Integration tests check if the multiservice-discovery lists all expected targets and their labels.
If not all targets are listed, or if some targets do not have the appropriate labels, we risk compromising the entire observability stack and the public dashboard.

## API spec

### `GET` /

Used for fetching the list of IC networks scraped by the multiservice discovery. The output looks like:

```JSON
[
    {
        "nns_urls": [
            "http://[2602:fb2b:100:10:5000:7cff:fe61:63ac]:8080/",
            "http://[2a00:fb01:400:42:5000:2bff:fe56:68d3]:8080/",
            "http://[2602:fb2b:100:10:5000:21ff:fea9:523b]:8080/",
            "http://[2a00:fb01:400:42:5000:aeff:fee0:fc5f]:8080/"
        ],
        "name": "benchmarkxsmall01",
        "public_key": null
    },
    {
        "nns_urls": [
            "http://[2a00:fb01:400:42:5000:aaff:fea4:ae46]:8080/",
            "http://[2602:fb2b:100:10:5000:3bff:febd:1a90]:8080/",
            "http://[2a00:fb01:400:42:5000:ecff:fe51:3c2e]:8080/",
            "http://[2602:fb2b:100:10:5000:44ff:fe7a:b0e1]:8080/"
        ],
        "name": "bitcoin",
        "public_key": null
    },
    {
        "nns_urls": [
            "http://[2607:f6f0:3004:1:5000:63ff:fe45:ad29]:8080/",
            "http://[2001:4d78:40d:0:5000:b0ff:feed:6ce8]:8080/",
            "http://[2602:fb2b:100:10:5000:bfff:feb7:d5dd]:8080/",
            "http://[2a00:fb01:400:42:5000:69ff:fe50:cfe]:8080/"
        ],
        "name": "cdhotfix01",
        "public_key": null
    },
]
```

### `POST` /

Used for registering one or more new IC networks for scraping by the multiservice discovery.
Use content type `application/json` and submit a body like this:

```JSON
{
    "nns_urls": [
        "http://[2602:fb2b:100:10:5000:7cff:fe61:63ac]:8080/",
        "http://[2a00:fb01:400:42:5000:2bff:fe56:68d3]:8080/",
        "http://[2602:fb2b:100:10:5000:21ff:fea9:523b]:8080/",
        "http://[2a00:fb01:400:42:5000:aeff:fee0:fc5f]:8080/"
    ],
    "name": "benchmarkxsmall01",
    "public_key": null
}
```

**NOTE**: The `name` field should be unique within the instance of the service discovery.

### `PUT` /

Replaces all known IC networks for scraping by the multiservice discovery with a new
list of IC networks. The content type is `application/json` with a content like:

```JSON
[
    {
        "nns_urls": [
            "http://[2602:fb2b:100:10:5000:7cff:fe61:63ac]:8080/",
            "http://[2a00:fb01:400:42:5000:2bff:fe56:68d3]:8080/",
            "http://[2602:fb2b:100:10:5000:21ff:fea9:523b]:8080/",
            "http://[2a00:fb01:400:42:5000:aeff:fee0:fc5f]:8080/"
        ],
        "name": "benchmarkxsmall01",
        "public_key": null
    },
    {
        "nns_urls": [
            "http://[2607:f6f0:3004:1:5000:63ff:fe45:ad29]:8080/",
            "http://[2001:4d78:40d:0:5000:b0ff:feed:6ce8]:8080/",
            "http://[2602:fb2b:100:10:5000:bfff:feb7:d5dd]:8080/",
            "http://[2a00:fb01:400:42:5000:69ff:fe50:cfe]:8080/"
        ],
        "name": "cdhotfix01",
        "public_key": null
    },
],
```

**NOTE**: The `name` field on each payload entry should be unique.

### `DELETE` /\<name\>

Used for deleting a network from the list of networks currently scraped by the service.

Example usage:

```sh
curl -X DELETE https://multiservice-discovery-url/<name>
```

### `GET` /targets

Used for retrieving a list of nodes available from all the scraping targets of the multiservice discovery. This is an open format that is intended to serve as a service. The intention is that whoever wants to consume this service should write his custom logic to map these targets to a format more suitable to them. The output is an array of objects where each looks like:

```JSON
[
    {
        "node_id": "o4j7n-2j2vj-xutgj-4n4it-xfnqw-o6gdr-zpumz-aaogx-znicu-bezl3-jqe",
        "ic_name": "benchmarkxsmall01", // This entry is linked to the scraping target named benchmarkxsmall01
        "targets": [
            "[2a00:fb01:400:42:5000:aeff:fee0:fc5f]:9090"
        ],
        "subnet_id": "xycig-kppcf-375z5-4j4jl-iubuv-3ppbl-z7vvn-36yj2-62c7u-c6urr-6ae",
        "dc_id": "",
        "operator_id": "5o66h-77qch-43oup-7aaui-kz5ty-tww4j-t2wmx-e3lym-cbtct-l3gpw-wae",
        "node_provider_id": "5o66h-77qch-43oup-7aaui-kz5ty-tww4j-t2wmx-e3lym-cbtct-l3gpw-wae",
        "jobs": [ // All jobs for the target
            "Replica",
            "Orchestrator",
            {
                "NodeExporter": "Guest"
            }
        ],
        "custom_labels": {},
        "name": "o4j7n-2j2vj-xutgj-4n4it-xfnqw-o6gdr-zpumz-aaogx-znicu-bezl3-jqe"
    },
]
```

### `GET` /prom/targets

Used for fetching all targets from service discovery in prometheus format which can be used as a prometheus target.

### `POST` /add_boundary_node

Used for adding boundary nodes to a certain scraping target. Since they are not in the registry and we need to tie them to a certain network this is the way. The body should look like:

```JSON
{
    "name": "bnp-00"
    "ic_name": "benchmarkxsmall01",
    "custom_labels": {
        "example": "value"
    },
    "targets": [
        "[2a00:fb01:400:42:5000:aeff:fee0:fc5f]:9090"
    ],
    "job_type": "job-type" //Accepted values: replica, orchestrator, node_exporter, host_node_exporter
}
```
