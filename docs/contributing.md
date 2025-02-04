# Contributing to DRE

Thank you for your interest in contributing to the Decentralized Reliability Engineering (DRE) project. This guide will help you set up your development environment and understand our contribution process.

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Project Structure](#project-structure)
3. [Development Workflow](#development-workflow)
4. [Running Tests](#running-tests)
5. [Submitting Changes](#submitting-changes)

## Development Environment Setup

### 1. Python Environment (Rye)

[Rye](https://rye.astral.sh/) is our preferred Python environment manager. It provides a unified experience for managing Python installations, dependencies, and virtual environments.

#### Installation

```bash
curl -sSf https://rye.astral.sh/get | bash
source "$HOME/.rye/env"  # Add to your shell's RC file
```

#### Project Setup

```bash
rye sync  # Install all dependencies
```

#### Common Rye Commands

```bash
rye run <command>  # Run a command with project dependencies
rye show           # Show current environment info
rye toolchain list --include-downloadable  # List available Python versions
```

### 2. IDE Configuration

Configure your IDE to use the Python interpreter from `.venv/bin/python`. This ensures consistent development settings across the team.

#### Troubleshooting Rye

If you encounter issues:
1. Update Rye: `rye self update`
2. Verify Python path: `which python3`
3. Check environment: `rye show`
4. List toolchains: `rye toolchain list --include-downloadable`

### 3. Pre-commit Hooks

We use pre-commit hooks to ensure code quality and consistency.

```bash
rye run pre-commit install
```

For more information, visit the [pre-commit documentation](https://pre-commit.com/#installation).

### 4. Rust Development Setup (Optional)

If you plan to work on Rust components:

#### Install Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### System Dependencies

For Linux:
```bash
sudo apt install -y clang mold protobuf-compiler
```

For macOS:
```bash
brew install mold protobuf
```

Add Cargo to your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"  # Add to your shell's RC file
```

#### Verify Rust Setup
```bash
cd rs
cargo check
```

### 5. Node.js and Yarn

Required for frontend development:

1. Install NVM:
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
```

2. Install Node.js:
```bash
nvm install 14
nvm use 14
```

3. Install Yarn:
```bash
npm install --global yarn
```

## Project Structure

The DRE repository is organized into several key components:

- `/dashboard` - Internal DRE dashboard (frontend and backend)
- `/rs` - Rust implementations
- `/pylib` - Python libraries
- `/docs` - Project documentation
- `/k8s` - Kubernetes configurations
- `/scripts` - Utility scripts

## Development Workflow

1. Create a new branch for your feature/fix
2. Make your changes
3. Ensure all tests pass
4. Submit a pull request

## Running Tests

### Backend Tests
```bash
rye run pytest
```

### Frontend Tests
```bash
cd dashboard
yarn test
```

## IC Network Internal Dashboard

### Setup
```bash
cd dashboard
yarn install
```

### Development
```bash
yarn dev  # Starts development server
```

### Using DRE CLI with Local Dashboard
```bash
dre --dev subnet replace --id <subnet-id> -o1
```

## Common Issues

### Linux: "No disk space left" with Bazel

If you encounter inotify issues:
```bash
sudo sysctl -w fs.inotify.max_user_watches=1048576
```

## Need Help?

- Check existing [GitHub Issues](https://github.com/dfinity/dre/issues)
- Join our developer community
- Review our [documentation](https://dfinity.github.io/dre/)
