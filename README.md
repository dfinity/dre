# Decentralized Reliability Engineering (DRE)

## Documentation in Github Pages

Searchable docs are available as GitHub pages at https://dfinity.github.io/dre/

## Installation

Please follow [getting started](docs/getting-started.md).

## Usage

In this repo we build:
* DRE cli tool
* Internal DRE dashboard, both frontend and backend
* Service discovery, which creates a list of IC targets for logs and metrics
* Log fetcher for IC nodes: Host, Guest, Boundary nodes
* Canister log fetcher
* Node Provider notifications, to notify node providers if node becomes unhealthy (unfinished and unmaintained code)

The DRE cli tool is built as an release artifact and published on GitHub: https://github.com/dfinity/dre/releases

Some examples of DRE cli tool usage are at [NNS proposals](nns-proposals.md), and elsewhere in the documentation. The documentation published on GitHub pages has quite good search, so please use that.

## Contributing

Please follow [contributing](docs/contributing.md).

## License

The contents of this repo are licensed under the [Apache 2 license](LICENSE).
