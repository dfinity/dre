# Pre-requisites

Install and enable pre-commit. Follow https://pre-commit.com/#installation then
```
pre-commit install
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

# Setup

Make sure to initialize the database before you run the
```
( cd "$(git rev-parse --show-toplevel)"; diesel --database-url $(. .env; echo $DATABASE_URL) setup )
```

```
( cd "$(git rev-parse --show-toplevel)"; diesel --database-url $(. .env; echo $DATABASE_URL) migration run )
```

# Flush and recreate the database

```
( cd "$(git rev-parse --show-toplevel)"; diesel --database-url $(. .env; echo $DATABASE_URL) database reset )
```
