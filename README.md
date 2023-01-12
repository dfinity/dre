# Pre-requisites

## 1. Install dependencies

### Linux

Installing SQLite with dev dependencies (tested on ubuntu 22.04).

``` bash
sudo apt install sqlite3 libsqlite3-dev
```

Note: This SQLite dependency must be installed before you install a python
version using pyenv, or the extension will not be compiled in your python. If 
you get an error like 

``` bash
ModuleNotFoundError: No module named '_sqlite3'
```

You need to remove your pyenv python version 

``` bash
pyenv uninstall <version>
```

and reinstall it again.

Installing pyenv to make it easier to manage python versions  (Tested on ubuntu
22.04 where the default python version is 3.10). You can use the [pyenv
installer](https://github.com/pyenv/pyenv-installer) to do it easily

``` bash
curl https://pyenv.run | bash
```

Installing Pipenv for Python by following https://pipenv.pypa.io/en/latest/#install-pipenv-today.
On Linux this may be as easy as running the following command in the repo root:
```bash
pip3 install --user pipenv
```

### MacOS install

On Mac, pipenv can be installed with Brew https://brew.sh/
```bash
brew install pipenv pyenv
```

If `pipenv shell` results in an error `configure: error: C compiler cannot create executables`,
you may not have recent development tools. Run the following:
```bash
sudo rm -rf /Library/Developer/CommandLineTools
sudo xcode-select --install
```

## 2. Install the Python packages

Run the following from the repo root
```bash
pipenv install
```

## 3. Install pre-commit
Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

## 4. Install cargo

You need an installation of `rustup` and `cargo`. You can follow the instructions from https://www.rust-lang.org/tools/install
This is typically as simple as running
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Make sure you add `$HOME/.cargo/bin` to your PATH, as written in the page above.
> In the Rust development environment, all tools are installed to the ~/.cargo/bin directory, and this is where you will find the Rust toolchain, including rustc, cargo, and rustup.

### Check the installation

To check if your Rust tooling is set up correctly, you can go to the repo root and then
```sh
cd rs
cargo check
```

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

To use release_cli with the local dashboard instance run it with `--dev` flag.

E.g.

```sh
release_cli --dev subnet --id <id> replace -o1
```

# Utils

## Updating the ic git submodule

This includes all git references, including the ones that were removed from the `ic` repo during the move from github to gitlab.
```
git submodule foreach -q git fetch origin
git submodule foreach -q git fetch github
git submodule foreach -q git fetch core-ic
git submodule foreach -q git checkout master
git submodule foreach -q git pull --prune --force
git submodule foreach -q git reset --hard origin/master
```
