# Node Providers notification

This binary is used to notify Node Providers of the Internet Computer of an issue 
with their nodes.

This service polls our metrics backend for the status of all the nodes currently being seen
by monitoring, and will send a webhook to the NPs defined in the file indicated by the 
$CONFIG_FILE_PATH_VAR_NAME. If this variable is not defined, no webhooks will be sent.

By default, all notifications will be logged using the Log Sink, which is currently hardcoded.

## Configuration

| Variable name | Required | Default |
|---|---|---|
| NNS_URL | True | None |
| NETWORK | True | None | 
| CONFIG_FILE_PATH_VAR_NAME | False | None |

### Configuration file format

See `example.conf.yaml`

## Running 

Running the server

``` shell
cargo run
```

Running tests

``` shell
cargo test
```
