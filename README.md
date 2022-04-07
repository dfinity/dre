# Pre-requisites

Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
```

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

