# Pre-requisites

Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

Install the nightly cargo. Needed for the new version of clap.
```
rustup toolchain install nightly
rustup override set nightly
```

Install the SQLite3 dev libraries.
```
sudo apt install libsqlite3-dev
```

# Checking

```
cargo check
```