POETRY_ENV_PATH := $(shell ( venv/bin/poetry env info ; echo Path: /tmp/PhonyPath ) | grep Path: | head -1 | awk -F :  '{ print $$2 }')
CARGO_PATH := $(shell which cargo || echo /tmp/PhonyCargo )

.PHONY = debug

debug:
	echo $(POETRY_ENV_PATH)
	echo $(CARGO_PATH)

venv/bin/pip3:
	virtualenv venv || { echo virtualenv is not installed or on your PATH, please install it. ; exit 1 ; }

venv/bin/poetry: venv/bin/pip3
	venv/bin/pip3 install poetry

$(POETRY_ENV_PATH): venv/bin/poetry
	venv/bin/poetry install --no-root

$(POETRY_ENV_PATH)/bin/poetry: $(POETRY_ENV_PATH) venv/bin/poetry
	venv/bin/poetry run pip3 install poetry

$(POETRY_ENV_PATH)/bin/safety: $(POETRY_ENV_PATH)
	venv/bin/poetry run pip3 install safety

safety: $(POETRY_ENV_PATH)/bin/safety
	venv/bin/poetry run safety check

# Run this to lock Poetry dependencies after an update on pyproject.toml.
requirements.txt: pyproject.toml poetry.lock $(POETRY_ENV_PATH)/bin/poetry bin/poetry-export.sh WORKSPACE.bazel
	venv/bin/poetry run poetry lock --no-update
	venv/bin/poetry run bin/poetry-export.sh

$(CARGO_PATH)-check:
	cargo install cargo-check || { echo Cargo is not installed or on your PATH, please install it. ; exit 1 ; }
	touch $(CARGO_PATH)-check

cargo-audit: $(CARGO_PATH)-check
	cargo audit
