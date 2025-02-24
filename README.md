# Decentralized Reliability Engineering (DRE)

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](rust-toolchain.toml)
[![Python](https://img.shields.io/badge/Python-3.x-blue.svg)](.python-version)
[![Bazel](https://img.shields.io/badge/Build-Bazel-43a047.svg)](.bazelversion)

A comprehensive suite of tools and services for managing and monitoring Internet Computer (IC) infrastructure.

## 📚 Table of Contents

- [Documentation](#documentation)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## 📖 Documentation

Comprehensive, searchable documentation is available at [dfinity.github.io/dre](https://dfinity.github.io/dre/)

The documentation includes:
- Detailed API references
- Usage examples
- Best practices
- Troubleshooting guides

## 🚀 Features

DRE provides a powerful set of tools and services:

### Core Components

- **DRE CLI Tool**: Command-line interface for interacting with IC infrastructure
  - Available as a pre-built binary on [GitHub Releases](https://github.com/dfinity/dre/releases)
  - Examples available in [NNS proposals documentation](nns-proposals.md)

- **DRE Dashboard**: Comprehensive monitoring and management interface
  - Frontend and backend components
  - Real-time infrastructure insights
  - Interactive management capabilities

### Monitoring & Logging

- **Service Discovery**: Automated IC target discovery for logs and metrics
- **Log Fetchers**:
  - Host node logs
  - Guest node logs
  - Boundary node logs
  - Canister logs

### Infrastructure Management

- **Node Provider Notifications**: Health monitoring system for IC nodes
  - Automated alerts for node health issues
  - Note: Currently in maintenance mode

## 🛠 Installation

1. Check the [prerequisites](docs/getting-started.md#prerequisites)
2. Follow our detailed [getting started guide](docs/getting-started.md)
3. Verify your installation

## 💻 Usage

The DRE CLI tool (version 0.5.9) provides various commands for managing IC infrastructure:

```bash
# View all available commands
dre --help

# Common commands:
dre network        # Network-wide management operations, such as healing all subnets
dre subnet         # Subnet management, such as replacing nodes in a subnet
dre governance     # Commands and actions related to the IC NNS governance, such as submitting NNS motion proposals
dre proposals      # Listing or analyzing submitted NNS proposals
dre nodes          # Node operations, such as removing nodes from the IC
dre registry       # Registry reading
dre get            # Wrapper around ic-admin get-* commands
dre propose        # Wrapper around ic-admin propose-* commands
dre firewall       # Submitting proposals for firewall updates
dre node-metrics   # Getting the trustworthy node metrics
dre update-authorized-subnets  # Automatically updating the list of public IC subnets, based on subnet utilization
dre neuron         # Neuron topping up and checking balance
```

### Authentication Options

- `--private-key-pem`: Path to private key file (PEM format)
- `--neuron-id`: Explicitly setting the Neuron ID for governance operations, overriding the autodetection
- `--network`: Target network (mainnet, staging, or testnet)
- `--ic-admin`: Custom path to ic-admin
- `--hsm-*`: Hardware Security Module configurations

### Additional Options

- `--verbose`: Print detailed information
- `--dry-run`: Simulate operations without execution
- `--offline`: Run operations offline when possible
- `-y, --yes`: Skip confirmation prompts

For more examples and detailed usage instructions:
- Browse the [documentation](https://dfinity.github.io/dre/)
- Check the [NNS proposals guide](nns-proposals.md)
- Use the documentation search feature for specific topics

## 🤝 Contributing

We welcome contributions! Please see our [contributing guide](docs/contributing.md) for details on:
- Code style and standards
- Development setup
- Testing requirements
- Pull request process

## 📄 License

This project is licensed under the [Apache License 2.0](LICENSE).

---
Built with ❤️ by the DFINITY Foundation
