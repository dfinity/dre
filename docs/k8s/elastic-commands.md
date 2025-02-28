Some parts of elasticsearch configuration still has to be done manually since there is a limited scripting funcionallity.

One off scripts are worth being done through [kibana development console](https://kibana.mainnet.dfinity.network/app/dev_tools#/console)

This document contains some snippets that can be useful while configuring the cluster or while debugging certain issues with elastic. Furthermore, there are some [grafana dashboards](https://grafana.dm1-es1.dfinity.network/dashboards/f/eear4emsbaww0c/) which can help the developers investigate certain issues.

## General cluster health

Via kibana development console:
```bash
GET _cluster/health
```
or there is a [panel on grafana](https://grafana.dm1-es1.dfinity.network/d/elastic-es-cluster-dashboard/elasticsearch-cluster?var-interval=5m&from=now-1h&to=now&var-cluster=es&var-instance=192.168.70.46:9114&var-name=$__all&refresh=5m&viewPanel=panel-53) indicating the same information.

## Cluster stats

Via kibana dev console:
```bash
GET _cluster/stats

GET _cat/indices?s=index

GET _cat/nodes?v&h=id,v,rp,dt,du,dup

GET _cat/nodes
```
a lot of these are also visible on [this grafana dashboard](https://grafana.dm1-es1.dfinity.network/d/elastic-es-cluster-dashboard/elasticsearch-cluster?var-interval=5m&from=now-1h&to=now&var-cluster=es&var-instance=192.168.70.46:9114&var-name=$__all&refresh=5m)

## Cluster settings

Via kibana dev console:
```bash
# Listing settings
GET _cluster/settings

# Updating settings
PUT _cluster/settings
{
  "persistent": {
    "cluster": {
      "routing": {
        "allocation": {
          "node_concurrent_incoming_recoveries": "3",
          "enable": "all",
          "node_concurrent_outgoing_recoveries": "3"
        }
      },
      "max_shards_per_node": "6000"
    },
    "indices": {
      "recovery": {
        "max_bytes_per_sec": "100mb"
      }
    }
  },
  "transient": {}
}
```
## Ingest pipelines

Since there are a lot of logs that are not valuable we've introduced a couple of ingest pipelines which drop certain logs.

To configure them use kibana dev console:
```bash
# Getting the current pipelines
GET _ingest/pipeline

# Our current pipeline settings:
PUT _ingest/pipeline/false-warn-consensus
{
  "processors": [
    {
      "drop": {
        "description": "Drop documents with unneeded warn from Consensus",
        "if": """
String msg = ctx['MESSAGE'];
if (msg != null) {
  if (msg.contains("Error removing artifact ConsensusMessageId")) {
    return true;
  }
}
return false;
"""
      }
    }
  ]
}

PUT _ingest/pipeline/useless-log-entries
{
  "processors": [
    {
      "drop": {
        "description": "Drop documents that bring no value",
        "if": """
String msg = ctx['MESSAGE'];
if (msg == null) {
  msg = ctx['message'];
}
if (msg == null) {
  return true;
}
String syslog_identifier = ctx['SYSLOG_IDENTIFIER'];
if (syslog_identifier != null) {
  if (syslog_identifier == "filebeat") {
    return true;
  }
  if (syslog_identifier == "systemd") {
    if (msg != null) {
      if (msg.startsWith("Starting ") || msg.startsWith("Finished ")) {
        return true;
      }
      if (msg.endsWith(".service: Succeeded.")) {
        return true;
      }
    }
  }
}
if (msg != null) {
  if (msg.startsWith("Drop - default policy: IN=enp1s0 OUT=")) {
    return true;
  }
  if (msg.startsWith("OpenTelemetry trace error occurred.")) {
    return true;
  }
  if (msg.startsWith("[DTS] Finished response callback ")) {
    return true;
  }
  if (msg.endsWith("Done fetching new response.")) {
    return true;
  }
  if (msg.trim().length() <= 5) {
    return true;
  }
}
return false;
"""
      }
    }
  ]
}

PUT _ingest/pipeline/nftables_log_entries
{
  "description": "Parses nftable log entries into ECS format, dynamically capturing the drop/reject reason",
  "processors": [
    {
      "grok": {
        "field": "MESSAGE",
        "patterns": [
          "^%{DATA:nftable_log_prefix}: IN=%{DATA:observer.ingress.interface.name} OUT=%{DATA:observer.egress.interface.name} MAC=%{DATA:source.mac} SRC=%{IPV6:source.ip} DST=%{IPV6:destination.ip} LEN=%{NUMBER:network.bytes} TC=%{NUMBER:network.traffic_class} HOPLIMIT=%{NUMBER:network.ttl} FLOWLBL=%{NUMBER:network.flow_label} PROTO=%{WORD:network.transport} SPT=%{NUMBER:source.port} DPT=%{NUMBER:destination.port} WINDOW=%{NUMBER:network.window} RES=%{DATA:network.res} SYN URGP=%{NUMBER:network.urgp} $"
        ],
        "ignore_missing": true
      }
    }
  ]
}

PUT _ingest/pipeline/selinux-audit-log
{
  "description": "Pipeline to process AVC log messages",
  "processors": [
    {
      "grok": {
        "field": "MESSAGE",
        "patterns": [
          "^AVC avc:  %{GREEDYDATA:action}  %{GREEDYDATA:auditd_rule} for  %{GREEDYDATA:kv_pairs}$"
        ],
        "ignore_failure": true
      }
    },
    {
      "script": {
        "lang": "painless",
        "source": "ctx.grok_success = ctx.containsKey('kv_pairs');",
        "ignore_failure": true
      }
    },
    {
      "gsub": {
        "if": "ctx.grok_success",
        "field": "kv_pairs",
        "pattern": "\"",
        "replacement": "",
        "ignore_failure": true
      }
    },
    {
      "kv": {
        "if": "ctx.grok_success",
        "field": "kv_pairs",
        "field_split": " ",
        "value_split": "=",
        "ignore_failure": true
      }
    },
    {
      "grok": {
        "if": "ctx.grok_success",
        "field": "scontext",
        "patterns": [
          "^%{DATA:source_context.user}:%{DATA:source_context.role}:%{DATA:source_context.type}:%{DATA:source_context.level}$"
        ],
        "ignore_failure": true
      }
    },
    {
      "grok": {
        "if": "ctx.grok_success",
        "field": "tcontext",
        "patterns": [
          "^%{DATA:target_context.user}:%{DATA:target_context.role}:%{DATA:target_context.type}:%{DATA:target_context.level}$"
        ],
        "ignore_failure": true
      }
    },
    {
      "remove": {
        "if": "ctx.grok_success",
        "field": "kv_pairs",
        "ignore_failure": true,
        "ignore_missing": true
      }
    }
  ]
}


PUT _ingest/pipeline/rm-noisy-log-entries
{
  "processors": [
    {
      "drop": {
        "if": """String msg = ctx['MESSAGE'];
if (msg == null) {
  msg = ctx['message'];
}
if (msg == null) {
  return true;
}
String syslog_identifier = ctx['SYSLOG_IDENTIFIER'];
if (syslog_identifier != null) {
  if (syslog_identifier == "filebeat") {
    return true;
  }
  if (syslog_identifier == "systemd") {
    if (msg != null) {
      if (msg.startsWith("Starting ") || msg.startsWith("Finished ")) {
        return true;
      }
      if (msg.endsWith(".service: Succeeded.")) {
        return true;
      }
      if (msg.endsWith(".service: Deactivated successfully.")) {
        return true;
      }
    }
  }
}
if (msg != null) {
  if (msg.startsWith("Drop - default policy: IN=enp1s0 OUT=")) {
    return true;
  }
  if (msg.startsWith("[DTS] Finished response callback")) {
    return true;
  }
  if (msg.startsWith("Received the response for HttpRequest with callback id")) {
    return true;
  }
  if (msg.endsWith("[Canister renrk-eyaaa-aaaaa-aaada-cai] [GTC] get_account")) {
    return true;
  }
  if (msg.contains("[Canister ryjl3-tyaaa-aaaaa-aaaba-cai] [ledger] Checking the ledger for block")) {
    return true;
  }
  if (msg.endsWith("Done fetching new response.")) {
    return true;
  }
  if (msg.trim().length() <= 5) {
    return true;
  }
}
return false;
""",
        "ignore_failure": true,
        "description": "Drop documents that bring no value"
      }
    }
  ]
}

PUT _ingest/pipeline/One-pipeline-to-rule-them-all
{
  "processors": [
    {
      "pipeline": {
        "name": "rm-noisy-log-entries",
        "ignore_failure": false
      }
    },
    {
      "pipeline": {
        "name": "nftables_log_entries",
        "ignore_failure": true
      }
    },
    {
      "pipeline": {
        "name": "selinux-audit-log"
      }
    },
    {
      "pipeline": {
        "name": "false-warn-consensus",
        "ignore_failure": true
      }
    }
  ]
}

# After creation of these pipelines devs should update the index template
PUT _index_template/ic_logs_index_template
{
  "priority": 2,
  "template": {
    "settings": {
      "index": {
        "routing": {
          "allocation": {
            "include": {
              "_tier_preference": "data_content"
            }
          }
        },
        "default_pipeline": "One-pipeline-to-rule-them-all",
        "refresh_interval": "30s",
        "number_of_shards": "6",
        "number_of_replicas": "1"
      }
    },
    "mappings": {
      "dynamic": true,
      "numeric_detection": false,
      "date_detection": true,
      "dynamic_date_formats": [
        "strict_date_optional_time",
        "yyyy/MM/dd HH:mm:ss Z||yyyy/MM/dd Z"
      ],
      "_source": {
        "enabled": true,
        "includes": [],
        "excludes": []
      },
      "_routing": {
        "required": false
      },
      "dynamic_templates": []
    }
  },
  "index_patterns": [
    "ic-logs-*"
  ]
}
```

## Some more tips and tricks

In kibana dev console:
```bash
# Update the default pipeline for one index
PUT ic-logs-2024-11-28/_settings
{
  "index": {
    "default_pipeline": "One-pipeline-to-rule-them-all"
  }
}

# Get the settings for one index
GET ic-logs-2024-11-28/_settings

# Reroute specific shards
POST _cluster/reroute
{
  "commands" : [
    {"move" : {
      "index" : "indexname",
      "shard" : 1,
      "from_node" : "nodename",
      "to_node" : "nodename"
      }
    }
  ]
}

# Reindex certain indices
POST _reindex
{
  "source": {
    "index": "ic-logs-2024-01-14"
  },
  "dest": {
    "index": "reindex-ic-logs-2024-01-14"
  }
}

# Delete indices
DELETE ic-logs-2024-08-01,ic-logs-2024-08-02

# Delete with wildcards
PUT _cluster/settings
{
    "transient": {
        "action.destructive_requires_name": false // allow wildcards
    }
}

DELETE ic-logs-2024-*

PUT _cluster/settings
{
    "transient": {
        "action.destructive_requires_name": true // disallow wildcards
    }
}
```
