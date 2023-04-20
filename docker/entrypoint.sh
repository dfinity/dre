#!/usr/bin/env bash
USER=ubuntu
exec gosu $USER "$@"
