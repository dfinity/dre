# Contributing to DRE

[![Contributions Welcome](https://img.shields.io/badge/contributions-welcome-brightgreen.svg)](https://github.com/dfinity/dre/issues)
[![Rye](https://img.shields.io/badge/python_manager-rye-blue)](https://rye.astral.sh/)
[![Pre-commit](https://img.shields.io/badge/pre--commit-enabled-brightgreen?logo=pre-commit&logoColor=white)](https://pre-commit.com/)

Thank you for your interest in contributing to the Decentralized Reliability Engineering (DRE) project. This guide will help you set up your development environment and understand our contribution process.

## 📚 Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Project Structure](#project-structure)
3. [Code Style Guidelines](#code-style-guidelines)
4. [Development Workflow](#development-workflow)
5. [Pull Request Process](#pull-request-process)
6. [Running Tests](#running-tests)
7. [Common Issues](#common-issues)
8. [Getting Help](#getting-help)

## 🛠 Development Environment Setup

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

## 📂 Project Structure

The DRE repository is organized into several key components:

- `/dashboard` - Internal DRE dashboard (frontend and backend)
- `/rs` - Rust implementations
- `/pylib` - Python libraries
- `/docs` - Project documentation
- `/k8s` - Kubernetes configurations
- `/scripts` - Utility scripts

## 📝 Code Style Guidelines

### Python
- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/) style guide
- Use type hints for function arguments and return values
- Document functions and classes using docstrings
- Maximum line length: 100 characters

### Rust
- Follow the official [Rust Style Guide](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for code formatting
- Run `clippy` for linting

### JavaScript/TypeScript
- Follow the project's ESLint configuration
- Use TypeScript for new code
- Follow the [Angular commit message format](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#commit)

## 🔄 Development Workflow

1. Fork the repository and create your branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Set up development environment:
   ```bash
   rye sync
   rye run pre-commit install
   ```

3. Make your changes:
   - Write tests for new functionality
   - Update documentation as needed
   - Follow code style guidelines

4. Commit your changes:
   ```bash
   git commit -m "feat: add new feature"
   ```
   Follow the [conventional commits](https://www.conventionalcommits.org/) specification

5. Push to your fork and create a pull request

## 🔍 Pull Request Process

1. Ensure all tests pass locally
2. Update documentation if needed
3. Add a clear description of the changes
4. Link any related issues
5. Request review from maintainers
6. Address review feedback
7. Ensure CI checks pass

### PR Title Format
- feat: Add new feature
- fix: Fix bug
- docs: Update documentation
- test: Add tests
- refactor: Code refactoring
- chore: Maintenance tasks

## ⚡ Running Tests

### Backend Tests
```bash
rye run pytest
```

### Frontend Tests
```bash
cd dashboard
yarn test
```

## 🖥 IC Network Internal Dashboard

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

## ❗ Common Issues

### Linux: "No disk space left" with Bazel

If you encounter inotify issues:
```bash
sudo sysctl -w fs.inotify.max_user_watches=1048576
```

### Other Common Issues

1. **Permission Denied Errors**
   ```bash
   sudo chown -R $(whoami) .
   ```

2. **Node Version Mismatch**
   ```bash
   nvm use 14  # Ensure correct Node version
   ```

3. **Bazel Cache Issues**
   ```bash
   bazel clean --expunge
   ```

## 🤝 Getting Help

- Check existing [GitHub Issues](https://github.com/dfinity/dre/issues)
- Join our developer community
- Review our [documentation](https://dfinity.github.io/dre/)
- Reach out to maintainers on Discord

### Before Asking for Help

1. Search existing issues
2. Check the documentation
3. Try troubleshooting steps
4. Provide relevant details when asking

---
Remember: Good code is not just about functionality—it's about maintainability, readability, and collaboration. Thank you for contributing to DRE! 🚀
