# Installing the pre-commit hook

Follow https://pre-commit.com/#installation then
```
pre-commit install
```

# Installing the nightly cargo

Needed for the new version of clap
```
rustup toolchain install nightly
```

# Checking

```
cargo +nightly check
```