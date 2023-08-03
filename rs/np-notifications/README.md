# Node Providers notification

This binary is used to notify Node Providers of the Internet Computer of an issue 
with their nodes.

To run this, you will need to have a Matrix account ready, with its username and password.
See below for configuring the credentials for matrix.

## Configuration

| Variable name | Required | Default |
|---|---|---|
| NP_MATRIX_USERNAME | True | None |
| NP_MATRIX PASSWORD | True | None | 
| NP_MATRIX_INSTANCE | False | "https://matrix.org" |

## Running 

Running the server

``` shell
cargo run
```

Running tests

``` shell
cargo test
```

Running tests that require external services

``` shell
export NP_MATRIX_ROOM_ID="<room id in which to receive the test message>"
cargo test -- --ignored
```
