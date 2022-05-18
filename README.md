# Pre-requisites

### Linux

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

# Checking

```
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
