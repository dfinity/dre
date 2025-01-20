# Pre-requisites

## Rye installation

Rye is a comprehensive project and package management solution for Python.
Rye provides a unified experience to install and manages Python installations,
pyproject.toml based projects, dependencies and virtualenvs seamlessly.

Run the following from the repo root:

```bash
curl -sSf https://rye.astral.sh/get | bash
```

Follow the instructions on screen. Once the install is done,
reopen your shell or run `source "$HOME/.rye/env"`.

You can make sure all dependencies are installed by running

```bash
rye sync
```

And you can enter the `venv` manually if needed by running `. .venv/bin/activate`.
This is typically not needed.

It's sufficient to prefix any command with the following:

```bash
rye run <command>
```

to run the `<command>` with all expected dependencies.

### IDE setup

Point your IDE to the Python interpreter inside `.venv/bin`.

### Troubleshooting rye

If you face problems in `rye sync`, such as `unknown version cpython@...`, you can try to

* List all available toolchains
```
rye toolchain list --include-downloadable
```

* Upgrade rye itself
```
rye self update
```

* Ensure rye python is in path
```
which python3
```

* Show the actively used rye environment in the project
```
rye show
```


### 3. Install pre-commit

Install and enable pre-commit. It's highly recommended in order to prevent pushing code to github that will surely cause failures.

```
# cd ~/src/dre
rye run pre-commit install
```

More detailed instructions at https://pre-commit.com/#installation .

### 4.a Install cargo (optional)

If you build with cargo, and not with bazel, you need an installation of `rustup` and `cargo`. You can follow the instructions from https://www.rust-lang.org/tools/install
This is typically as simple as running

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
#### On Linux
```sh
command -v apt && sudo apt install -y clang mold protobuf-compiler || true
```
#### On Mac OS
No need to install Clang for Mac OS user since it comes with Xcode.
```sh
brew install mold protobuff
```
Make sure you add `$HOME/.cargo/bin` to your PATH, as written in the page above.
> In the Rust development environment, all tools are installed to the ~/.cargo/bin directory, and this is where you will find the Rust toolchain, including rustc, cargo, and rustup.

### Check the Rust / Cargo installation

To check if your Rust tooling is set up correctly, you can go to the repo root and then
```sh
cd rs
cargo check
```

This should succeed.

### 4.b Install bazel

To install bazel, do not use the version provided by your OS package manager. Please make sure you use [bazelisk](https://bazel.build/install/bazelisk).

## 5. Install nvm, node, yarn

### 1. Install nvm

https://github.com/nvm-sh/nvm#installing-and-updating

### 2. Install node

```sh
nvm install 14
nvm use 14
```

### 3. Install yarn

```sh
npm install --global yarn
```

### "No disk space left" when building with Bazel on Linux?

```
sudo sysctl -w fs.inotify.max_user_watches=1048576
```

Bazel eats up a lot of inotify user watches.

# IC Network Internal Dashboard

## Pre-requisites

### 1. Install cargo-watch

```sh
cargo install cargo-watch
```

### 2. Install yarn dependencies

```
cd dashboard
yarn install
```

## Running

To start the release dashboard locally, run the following from dashboard folder

```sh
yarn dev
```

To use the `dre` CLI tool with the local dashboard instance run it with `--dev` flag.

E.g.

```sh
dre --dev subnet replace --id <subnet-id> -o1
```
