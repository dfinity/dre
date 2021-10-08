# Pre-requisites

Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

Install the nightly cargo. Needed for the new version of clap.
```
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

# Checking

```
cargo check
```
