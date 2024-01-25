# Pre-requisites

## 1. Install dependencies

### pixi

[Pixi](https://pixi.sh/) is a package management tool for developers. It allows the developer to install libraries and applications in a reproducible way. Use pixi cross-platform, on Windows, Mac and Linux.

Installation:
```
curl -fsSL https://pixi.sh/install.sh | bash
```

Then logout and login and you can then install Python with:
```
pixi global install python==3.11
```

### pyenv

pyenv is a more conventional alternative to pixi. It installs slower but it's more tested. Use it if pixi doesn't work for you.

#### On Linux

In order to manage python versions, you can use the [pyenv
installer](https://github.com/pyenv/pyenv-installer).

Installing pyenv would be something like:

``` bash
curl https://pyenv.run | bash
```

Then log off and log back on, in order to ensure that the
`~/.local/bin` directory (used by `pip`) is
available in your session's `$PATH`, as well as the pyenv
shims directory.

#### On Mac OS

On Mac, pyenv can be installed with Brew https://brew.sh/
```bash
brew install pyenv
```

If you get an error `configure: error: C compiler cannot create executables`,
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
pyenv install 3.11.6  # installs Python 3.11.6 via pyenv
pyenv local 3.11.6    # tells pyenv to use 3.11.6 for this repo
pip3 install poetry   # installs poetry to your 3.11.6
poetry env use $(which python)  # instructs poetry to use 3.11.6
poetry install        # installs all our dependencies to 3.11.6
```

Follow the instructions onscreen.  Once the install is done,
close and open your shell window, or run `bash` again.
When you change into the `release` directory (this repo),
typing `poetry env info` should show that the current
folder is associated with a 3.11-based virtualenv.

You can see the full path to your virtualenv's Python interpreter
with the command `poetry env info -p`.  This is the interpreter
you should use in your IDE and in day-to-day commands with regards
to the Python programs in this repo.  To activate the use of
this interpreter on the shell:

```bash
source "$(poetry env info -p)/bin/activate"
```

### 3. Install pre-commit

Install and enable pre-commit. It's highly recommended in order to prevent pushing code to github that will surely cause failures.

```
# cd ~/src/release
# source "$(poetry env info -p)/bin/activate"
pip3 install --user pre-commit
pre-commit install
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
dre --dev subnet --id <id> replace -o1
```
