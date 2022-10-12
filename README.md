# Pre-requisites

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

### Installing the Python packages (dependencies)

Run the following from the repo root
```bash
pipenv install
```

### Pre-commit
Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

## Adding the ic git submodule

Add the `ic` git submodule
```
git submodule update --init --recursive --remote
git submodule foreach -q git remote add github git@github.com:dfinity-lab/dfinity.git
git submodule foreach -q git remote add core-ic git@gitlab.com:dfinity-lab/core/ic.git
```

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

## Installing cargo

You need an installation of `rustup` and `cargo`. You can follow the instructions from https://www.rust-lang.org/tools/install
This is typically as simple as running
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Make sure you add `$HOME/.cargo/bin` to your PATH, as written in the page above.
> In the Rust development environment, all tools are installed to the ~/.cargo/bin directory, and this is where you will find the Rust toolchain, including rustc, cargo, and rustup.

# Checking

To check if your Rust tooling is set up correctly, you can go to the repo root and then
```
cd rs
cargo check
```

# [Backstage](https://backstage.io)

To start the release dashboard locally, run:

```sh
cargo install cargo-watch
cd dashboard
yarn install
yarn dev
```
