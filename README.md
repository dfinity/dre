# Documentation in Github Pages

Searchable docs are available as GitHub pages at https://dfinity.github.io/dre/

# Pre-requisites

## 1. Install dependencies

### pipenv / pyenv

#### On Linux

Install pyenv to make it easier to manage python versions  (Tested on ubuntu
22.04 where the default python version is 3.10). You can use the [pyenv
installer](https://github.com/pyenv/pyenv-installer) to do it easily, or go
as simple as:

``` bash
curl https://pyenv.run | bash
```

Then log off and log back on, in order to ensure that the
`~/.local/bin` directory (used by `pip` and `pipenv`) is
available in your session's `$PATH`, as well as the pyenv
shims directory.

#### On Mac OS

On Mac, pipenv can be installed with Brew https://brew.sh/
```bash
brew install pyenv
```

If `pipenv shell` results in an error `configure: error: C compiler cannot create executables`,
you may not have recent development tools. Run the following:
```bash
sudo rm -rf /Library/Developer/CommandLineTools
sudo xcode-select --install
```

You should verify that a new terminal session has added
the pyenv shims directory to your `$PATH`, then continue
in that new terminal session from now on.

### 2. Install the Python packages needed by the repo


#### Linux dependencies

pyenv will install a clean Python for you.   This installation will
insist on a few important libraries which you should have on your
system before it installs our chosen Python development version.

```bash
sudo apt install -y libncurses-dev libbz2-dev libreadline-dev \
  libssl-dev make build-essential libssl-dev zlib1g-dev \
  libbz2-dev libreadline-dev libsqlite3-dev wget curl llvm \
  libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev \
  libffi-dev liblzma-dev
```

Note: if the list of dependencies above changes, update the
[docker/Dockerfile] file accordingly, so CI stays in sync
with local development environments.

#### poetry installation

Run the following from the repo root:

```bash
# change into the directory of the repo
# cd ~/src/release
pyenv install 3.8.16  # installs Python 3.8.16 via pyenv
pyenv local 3.8.16    # tells pyenv to use 3.8.16 for this repo
pip3 install poetry   # installs poetry to your 3.8.16
poetry env use $(which python)  # instructs poetry to use 3.8.16
poetry install        # installs all our dependencies to 3.8.16
```

Follow the instructions onscreen.  Once the install is done,
close and open your shell window, or run `bash` again.
When you change into the `release` directory (this repo),
typing `poetry env info` should show that the current
folder is associated with a 3.8-based virtualenv.

Should problems arise during the install, you'll have to remove
the environment poby running `pipenv --rm`.

You can see the full path to your virtualenv's Python interpreter
with the command `poetry env info -p`.  This is the interpreter
you should use in your IDE and in day-to-day commands with regards
to the Python programs in this repo.  To activate the use of
this interpreter on the shell:

```bash
source "$(poetry env info -p)/bin/activate"
```

### 3. Install pre-commit

Install and enable pre-commit.

```
# cd ~/src/release
# source "$(poetry env info -p)/bin/activate"
pip3 install --user pre-commit
pre-commit install
```

More detailed instructions at https://pre-commit.com/#installation .

### 4. Install cargo

You need an installation of `rustup` and `cargo`. You can follow the instructions from https://www.rust-lang.org/tools/install
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

# CI container builds

This repository creates a container that is used in CI.

To build this container locally:

```
# cd ~/src/release
python3 docker/docker-update-image.py
# To diagnose *just* the build, run:
#   docker build -f docker/Dockerfile .
# in the root of the repository.
```

You can export variables `BUILDER=buildah` and `CREATOR=podman` to use
Podman and Buildah during builds, instead of Docker.  To make Buildah
use intermediate layers -- speeds up unchanged intermediate steps --
simply export `BUILDAH_LAYERS=true`.

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
dre --dev subnet --id <id> replace -o1
```
