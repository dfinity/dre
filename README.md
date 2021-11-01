# Pre-requisites

Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

Install the nightly cargo and enable it in the project folder. Needed for the new version of clap.
```
cd <project_dir>
which rustup || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup override set nightly
```

Install the SQLite3 dev libraries.
```
sudo apt install libsqlite3-dev
```

Install the SQLite3 ORM
```
cargo install diesel_cli --no-default-features --features sqlite
```

Add the `ic` git submodule
```
git submodule update --init --recursive --remote
git submodule foreach -q git remote add github git@github.com:dfinity-lab/dfinity.git
```

## Updating the ic git submodule

This includes all git references, including the ones that were removed from the `ic` repo during the move from github to gitlab.
```
git submodule foreach -q git fetch github
git submodule foreach -q git checkout master
git submodule foreach -q git pull --prune --force
git submodule foreach -q git reset --hard origin/master
```

# Checking

```
cargo check
```
